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

use std::cell::RefCell;
use std::rc::Rc;

use gio;
use gio::prelude::*;
use glib;
//use glib::prelude::*;
//use gtk;
use gtk::prelude::*;

use crate::arcam_protocol;
use crate::control_window::ControlWindow;
use crate::functionality;

/*
 * ================================================================================
  *
 *  Proposal from Sebastian Dröge  to provide a more Rust-y API to GIO sockets in gtk-rs.
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

#[derive(Debug)]
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

pub async fn initialise_socket_and_listen_for_packets_from_amp(control_window: Rc<ControlWindow>, address: String, port_number: u16) {
    eprintln!("$$$$  initialise_socket_and_listen_for_packets_from_amp: trying to connect to {}:{}", address, port_number);
    let socket_client = SocketClient::new();
    match socket_client.connect(&gio::NetworkAddress::new(address.as_ref(), port_number)).await {
        Ok(s) => *control_window.socket_connection.borrow_mut() = Some(Rc::new(RefCell::new(s))),
        Err(_) => {
            //  TODO Must remove all this UI stuff from what should be the comms stuff.
            eprintln!("$$$$  initialise_socket_and_listen_for_packets_from_amp: failed to connect to {}:{}", address, port_number);
            let dialogue = gtk::MessageDialog::new(
                Some(&control_window.window),
                gtk::DialogFlags::MODAL,
                gtk::MessageType::Info,
                gtk::ButtonsType::Ok,
                &format!("Failed to connect to {}:{}", address, port_number),
            );
            dialogue.run();
            dialogue.destroy();
            if control_window.connect.get_active() { control_window.connect.set_active(false); };
            assert!(control_window.socket_connection.borrow().is_none());
            return;
        },
    };
    eprintln!("$$$$  initialise_socket_and_listen_for_packets_from_amp: connected to {}:{}", address, port_number);
    if !control_window.connect.get_active() { control_window.connect.set_active(true); }
    let mut queue: Vec<u8> = vec![];
    let mut buffer = [0u8; 256];
    eprintln!("$$$$  initialise_socket_and_listen_for_packets_from_amp: entering listen loop for {}:{}", address, port_number);
    loop {
        //  TODO Find a way of having the blocking read without keeping the mutable borrow open.
        let count = match control_window.socket_connection.borrow().as_ref().unwrap().borrow_mut().read(&mut buffer).await {
           Ok(s) => {
               eprintln!("$$$$  initialise_socket_and_listen_for_packets_from_amp: got a packet: {:?}", &buffer[..s]);
               s
           },
            Err(e) => {
                eprintln!("$$$$  initialise_socket_and_listen_for_packets_from_amp: failed to read.");
                0
            },
        };
        if count == 0 { break; }
        for i in 0..count {
            queue.push(buffer[i]);
        }
        eprintln!("$$$$  initialise_socket_and_listen_for_packets_from_amp: pushed values nto queue: {:?}", &queue);
        match arcam_protocol::parse_response(&queue) {
            Ok((zone, cc, ac, data, count)) => {
                eprintln!("$$$$  initialise_socket_and_listen_for_packets_from_amp: got a successful parse of a packet.");
                for _ in 0..count { queue.pop(); }
                functionality::process_response(&control_window, zone, cc, ac, &data);
            },
            Err(e) => {
                eprintln!("$$$$  initialise_socket_and_listen_for_packets_from_amp: failed to parse a packet.");
                match e {
                    "Insufficient bytes to form a packet." => {},
                    _ => panic!("XXXXX {}", e),
                }
            },
        }
    }
    *control_window.socket_connection.borrow_mut() = None;
    if control_window.connect.get_active() { control_window.connect.set_active(false); };
}

/// Terminate the current connection.
pub async fn terminate_connection(control_window: Rc<ControlWindow>) {
    if (*control_window.socket_connection.borrow_mut()).is_some() {
        eprintln!("$$$$  terminate_connection: closing current connection.");
        match control_window.socket_connection.borrow().as_ref().unwrap().borrow_mut().close().await {
            Ok(s) => {},
            Err(e) => eprintln!("$$$$  terminate_connection: failed to close the connection: {:?}", e),
        };
    } else {
        eprintln!("$$$$  terminate_connection: attempted to close a not open connection.");
    };
}

pub async fn send_to_amp(control_window: Rc<ControlWindow>, packet: Vec<u8>) {
    eprintln!("$$$$  send_to_amp: send packet to amp {:?}", packet);
    match control_window.socket_connection.borrow().as_ref().unwrap().borrow_mut().write_all(&packet).await {
        Ok(s) => {},
        Err(e) => eprintln!("$$$$  send_to_amp: failed to send ot the amp on the connection: {:?}", e),
    }
}
