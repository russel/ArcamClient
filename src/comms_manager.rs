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

use futures;
use futures::AsyncReadExt;
use futures::AsyncWriteExt;
use futures::StreamExt;

use crate::socket_support::{SocketClient, SocketConnection};

async fn listen_to_reader(
    mut reader: futures::io::ReadHalf<SocketConnection>,
    from_comms_manager: glib::Sender<Vec<u8>>
) {
    // TODO should the byte sequence parsing happen here or elsewhere?
    let mut queue: Vec<u8> = vec![];
    let mut buffer = [0u8; 256];
    eprintln!("comms_manager::listen_to_reader: entering listen loop");
    loop {
        // TODO How to disconnect this listener when the connection is closed?
        let count = match reader.read(&mut buffer).await {
            Ok(s) => {
                eprintln!("comms_manager::listen_to_reader: got a packet: {:?}", &buffer[..s]);
                s
            },
            Err(e) => {
                eprintln!("comms_manager::listen_to_reader: failed to read – {:?}", e);
                0
            },
        };
        //  TODO what happens if the amp is switched off (or put to sleep) during a connection?
        if count == 0 { break; }
        match from_comms_manager.send(buffer[..count].to_vec()) {
            Ok(_) => {},
            Err(e) => eprintln!("comms_manager::listen_to_reader: failed to send packet – {:?}.", e),
        };
    }
}

async fn start_a_connection_and_set_up_event_listeners(
    to_control_window: glib::Sender<Vec<u8>>,
    mut to_comms_manager: futures::channel::mpsc::Receiver<Vec<u8>>,
    address: gio::NetworkAddress,
) {
    eprintln!("comms_manager::start_a_connection_and_set_up_event_listeners: setting up connection to {}:{}", address.get_hostname().unwrap(), address.get_port());
    let client = SocketClient::new();
    let connection = match client.connect(&address).await {
        Ok(s) => {
            eprintln!("comms_manager::start_a_connection_and_set_up_event_listeners: connected to {}:{}", address.get_hostname().unwrap(), address.get_port());
            s
        },
        Err(_) => {
            eprintln!("comms_manager::start_a_connection_and_set_up_event_listeners: failed to connect to {}:{}", address.get_hostname().unwrap(), address.get_port());
            return
        },
    };
    let (reader, mut writer) = connection.split();
    let context = glib::MainContext::default();
    context.spawn_local(async move {
        while let Some(data) = to_comms_manager.next().await {
            eprintln!("comms_manager::start_a_connection_and_set_up_event_listeners: writing {:?}", &data);
            match writer.write_all(&data).await {
                Ok(_) => { eprintln!("comms_manager::start_a_connection_and_set_up_event_listeners: successfully sent packet to amp {:?}", data) },
                Err(e) => { eprintln!("comms_manager::start_a_connection_and_set_up_event_listeners: error sending packet to amp {:?}", e) },
            };
        }
    });
    context.spawn_local(listen_to_reader(reader, to_control_window));
    eprintln!("comms_manager::start_a_connection_and_set_up_event_listeners: set up connection to {:?}", address);
}

/// Connect to an Arcam amp at the address given.
pub fn connect_to_amp(
    to_control_window: &glib::Sender<Vec<u8>>,
    address: &str,
    port_number: u16
) -> Result<futures::channel::mpsc::Sender<Vec<u8>>, String> {
    // TODO This appears to always connect when in fact it doesn't.
    //   Need to find a way of messaging the functionality and control_window as to
    //   whether a connection was actually made or not.
    eprintln!("comms_manager::connect_to_amp: connecting to {:?}:{:?}", address, port_number);
    let (tx_to_comms_manager, rx_to_comms_manager) = futures::channel::mpsc::channel(10);
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

}
