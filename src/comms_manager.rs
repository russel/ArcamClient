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

use std::error::Error;

use gio;
use gio::prelude::*;
//use glib;
//use glib::prelude::*;
//use gtk;
//use gtk::prelude::*;

struct CommsManager {
    client: gio::SocketClient,
    connection: gio::TcpConnection,
    ostream: gio::PollableOutputStream,
    istream: gio::PollableInputStream,
}

pub fn send_to_amp(packet: &[u8]) {
    eprintln!("Send packet to amp {:?}", packet);
}

/// Create a future to put on the GTK event loop to handle all communications with
/// the connected to amplifier.
pub async fn make_connection(address: &str, port_number: u16) -> Result<(), Box<dyn Error>> {
    eprintln!("Connecting to {:?}", address);
    let client = gio::SocketClient::new();
    let connectable = gio::NetworkAddress::new(address, port_number);
    let connection = client.connect_async_future(&connectable).await?;
    let connection = connection.downcast::<gio::TcpConnection>().unwrap();
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

    Ok(())
}

/// Terminate the current connection.
pub fn terminate_connection() {
}


