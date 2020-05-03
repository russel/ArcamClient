/*
 *  arcamclient —  A gtk-rs based Rust application for controlling Arcam amplifiers.
 *
 *  Copyright © 2020  Russel Winder
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

use gio;
use gio::prelude::*;
use glib;
//use glib::prelude::*;
//use gtk;
//use gtk::prelude::*;

use futures;
//use futures::Future;

use crate::arcam_protocol::{AnswerCode, Command, ZoneNumber, parse_response};

/*
 * ================================================================================
 *
 *  Proposal from Sebastian Dröge  to provide a more Rust-y API to GIO sockets in gtk-rs.
 *  See  https://github.com/gtk-rs/gio/issues/293
 */

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{AsyncRead, AsyncWrite};
use futures_util::io::AsyncReadExt;
use futures_util::io::AsyncWriteExt;

pub struct SocketClient(gio::SocketClient);

impl SocketClient {
    pub fn new() -> Self {
        SocketClient(gio::SocketClient::new())
    }

    pub async fn connect<P: IsA<gio::SocketConnectable> + Clone + 'static>(
        &self,
        connectable: &P,
    ) -> Result<SocketConnection, glib::Error> {
        let connection = self.0.connect_async_future(connectable).await?;
        let ostream = connection
            .get_output_stream()
            .unwrap()
            .dynamic_cast::<gio::PollableOutputStream>()
            .unwrap();
        let write = ostream.into_async_write().unwrap();
        let istream = connection
            .get_input_stream()
            .unwrap()
            .dynamic_cast::<gio::PollableInputStream>()
            .unwrap();
        let read = istream.into_async_read().unwrap();
        Ok(SocketConnection{
            connection,
            read,
            write,
        })
    }
}

pub struct SocketConnection {
    connection: gio::SocketConnection,
    read: gio::InputStreamAsyncRead<gio::PollableInputStream>,
    write: gio::OutputStreamAsyncWrite<gio::PollableOutputStream>,
}

impl AsyncRead for SocketConnection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut Pin::get_mut(self).read).poll_read(cx, buf)
    }
}

impl AsyncWrite for SocketConnection {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut Pin::get_mut(self).write).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut Pin::get_mut(self).write).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Pin::new(&mut Pin::get_mut(self).write).poll_close(cx)
    }
}

/*
 *  End of proposal.
 *
 * ================================================================================
 */

async fn listen_to_reader(
    mut reader: futures::io::ReadHalf<SocketConnection>,
    from_comms_manager: glib::Sender<(ZoneNumber, Command, AnswerCode, Vec<u8>)>
) {
    // TODO should the byte sequence parsing happen here of elsewhere?
    let mut queue: Vec<u8> = vec![];
    let mut buffer = [0u8; 256];
    eprintln!("$$$$  listen_to_reader: entering listen loop");
    loop {
        let count = match reader.read(&mut buffer).await {
            Ok(s) => {
                eprintln!("$$$$  listen_to_reader: got a packet: {:?}", &buffer[..s]);
                s
            },
            Err(e) => {
                eprintln!("$$$$  listen_to_reader: failed to read – {:?}", e);
                0
            },
        };
        //  TODO what happens if the amp is switched off (or put to sleep) during a connection?
        if count == 0 { break; }
        for i in 0..count {
            queue.push(buffer[i]);
        }
        eprintln!("$$$$  listen_to_reader: pushed values nto queue: {:?}", &queue);
        match parse_response(&queue) {
            Ok((zone, cc, ac, data, count)) => {
                eprintln!("$$$$  listen_to_reader: got a successful parse of a packet.");
                for _ in 0..count { queue.pop(); }
                match from_comms_manager.send((zone, cc, ac, data)) {
                    Ok(_) => {},
                    Err(e) => eprintln!("$$$$  listen_to_reader: failed to send packet – {:?}.", e),
                }
            },
            Err(e) => {
                eprintln!("$$$$  listen_to_reader: failed to parse a packet.");
                match e {
                    "Insufficient bytes to form a packet." => {},
                    _ => panic!("XXXXX {}", e),
                }
            },
        }
    }
}

async fn start_a_connection_and_set_up_event_listeners(
    to_control_window: glib::Sender<(ZoneNumber, Command, AnswerCode, Vec<u8>)>,
    to_comms_manager: glib::Receiver<Vec<u8>>,
    address: gio::NetworkAddress,
) {
    let client = SocketClient::new();
    let connection = match client.connect(&address).await {
        Ok(s) => { s },
        Err(_) => { eprintln!("$$$$  start_a_connection_and_set_up_event_listeners: failed to connect to {}", address); return },
    };
    let (reader, mut writer) = connection.split();
    to_comms_manager.attach(None, |datum| {
        eprintln!("$$$$  start_a_connection_and_set_up_event_listeners: writing {:?}", &datum);
        /*
        match writer.write_all(&datum).poll() {
            futures::task::Poll::Ready(x) => {
                match x {
                    Ok(s) => {},
                    Err(e) => { eprintln!("$$$$  start_a_connection_and_set_up_event_listeners: error sending packet to amp {:?}", e) }
                }
            },
            futures::task::Poll::Pending => { eprintln!("$$$$  start_a_connection_and_set_up_event_listeners: not ready to send packet to amp") },
        };
         */
        /*
        let d = datum.clone();
        let w = writer.clone();
        let code = async move {
            match w.write_all(&d).await {
                Ok(_) => {},
                Err(e) => { eprintln!("$$$$  start_a_connection_and_set_up_event_listeners: error sending packet to amp {:?}", e) },
            };
        };
        glib::MainContext::default().spawn_local(code);
         */
        Continue(true)  //  TODO  Can terminate this after last read
    });
    glib::MainContext::default().spawn_local(listen_to_reader(reader, to_control_window));
}

/// Connect to an Arcam amp at the address given.
pub fn connect_to_amp(
    to_control_window: &glib::Sender<(ZoneNumber, Command, AnswerCode, Vec<u8>)>,
    address: &str,
    port_number: u16
) -> Result<glib::Sender<Vec<u8>>, String> {
    let (tx_to_comms_manager, rx_to_comms_manager) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);
    glib::MainContext::default().spawn_local(
        start_a_connection_and_set_up_event_listeners(
            to_control_window.clone(),
            rx_to_comms_manager,
            gio::NetworkAddress::new(address, port_number),
        )
    );
    Ok(tx_to_comms_manager)
}

/// Terminate the current connection.
pub fn disconnect_from_amp() {
    /*
    if (*control_window.socket_connection.borrow_mut()).is_some() {
        eprintln!("$$$$  terminate_connection: closing current connection.");
        match control_window.socket_connection.borrow().as_ref().unwrap().borrow_mut().close().await {
            Ok(s) => {},
            Err(e) => eprintln!("$$$$  terminate_connection: failed to close the connection: {:?}", e),
        };
    } else {
        eprintln!("$$$$  terminate_connection: attempted to close a not open connection.");
    };
    */
}
