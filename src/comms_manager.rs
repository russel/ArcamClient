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

//! This module provides mechanisms for sending bytes to an amplifier and receiving bytes from
//! an amplifier.
//!
//! Everything here is asynchronous communication with the amplifier. Communication into here
//! and out of here is using channels. The channel for sending bytes from the amplifier to other
//! code is provided via a call to the [connect_to_amp](fn.connect_to_amp.html) which returns
//! the channel for sending bytes to the amplifier.
//!
//! There is no knowledge of the Arcam protocol here, everything is just byte sequences. The
//! module [functionality](../functionality/index.html) has the functions that transform Arcam
//! [Requests](../arcam_protocol/struct.Request.html) into byte sequences that can be sent by by
//! code here and the functions that parse byte sequences into Arcam
//! [Response](../arcam_protocol/struct.Response.html)s.

use gio;
use gio::prelude::*;
use glib;
//use glib::prelude::*;

use futures;
use futures::AsyncReadExt;
use futures::AsyncWriteExt;
use futures::StreamExt;

use log::debug;

use gio_futures::{SocketClient, SocketConnection};

async fn listen_to_reader(
    mut reader: futures::io::ReadHalf<SocketConnection>,
    from_comms_manager: glib::Sender<Vec<u8>>
) {
    // TODO should the byte sequence parsing happen here or elsewhere?
    let mut buffer = [0u8; 256];
    debug!("listen_to_reader:  Entering listen loop.");
    loop {
        // TODO How to disconnect this listener when the connection is closed?
        let count = match reader.read(&mut buffer).await {
            Ok(s) => {
                debug!("listen_to_reader:  Got a packet: {:?}.", &buffer[..s]);
                s
            },
            Err(e) => {
                debug!("listen_to_reader:  Failed to read – {:?}.", e);
                0
            },
        };
        //  TODO what happens if the amp is switched off (or put to sleep) during a connection?
        if count == 0 { break; }
        match from_comms_manager.send(buffer[..count].to_vec()) {
            Ok(_) => {},
            Err(e) => debug!("listen_to_reader:  Failed to send packet – {:?}.", e),
        };
    }
}

async fn start_a_connection_and_set_up_event_listeners(
    to_control_window: glib::Sender<Vec<u8>>,
    mut to_comms_manager: futures::channel::mpsc::Receiver<Vec<u8>>,
    address: gio::NetworkAddress,
) {
    debug!("start_a_connection_and_set_up_event_listeners:  Setting up connection to {}:{}.", address.get_hostname().unwrap(), address.get_port());
    let client = SocketClient::new();
    let connection = match client.connect(&address).await {
        Ok(s) => {
            debug!("start_a_connection_and_set_up_event_listeners:  Connected to {}:{}.", address.get_hostname().unwrap(), address.get_port());
            s
        },
        Err(_) => {
            debug!("start_a_connection_and_set_up_event_listeners:  Failed to connect to {}:{}.", address.get_hostname().unwrap(), address.get_port());
            return
        },
    };
    let (reader, mut writer) = connection.split();
    let context = glib::MainContext::default();
    context.spawn_local(async move {
        while let Some(data) = to_comms_manager.next().await {
            debug!("start_a_connection_and_set_up_event_listeners:  Writing {:?}", &data);
            match writer.write_all(&data).await {
                Ok(_) => { debug!("start_a_connection_and_set_up_event_listeners:  Successfully sent packet to amp {:?}.", data); },
                Err(e) => {
                    // TODO Must think about showing disconnection in the UI when this happens.
                    debug!("start_a_connection_and_set_up_event_listeners:  Error sending packet to amp {:?}.", e);
                },
            };
        }
    });
    context.spawn_local(listen_to_reader(reader, to_control_window));
    debug!("start_a_connection_and_set_up_event_listeners:  Set up connection to {:?}.", address);
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
    debug!("connect_to_amp:  Connecting to {:?}:{:?}.", address, port_number);
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
