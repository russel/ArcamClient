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

mod start_avr850;

use std::io::{Write, Read};
use std::net::{SocketAddr, TcpStream};
use std::str::from_utf8;

use arcamclient::arcam_protocol::{
    ZoneNumber, Command, AnswerCode,
    create_request, parse_response
};

fn connect_mock_avr850_send_and_receive(send_data: &[u8]) -> Result<Vec<u8>, String> {
    let port_number: u16 = unsafe { start_avr850::portNumber };
    match TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port_number))) {
        Ok(mut stream) => {
            match stream.write(send_data) {
                Ok(send_count) => {
                    if send_count != send_data.len() { return Err("Failed to write the correct number of bytes to the mock AVR850".to_string()); }
                    let mut buffer = [0; 256];
                    match stream.read(&mut buffer) {
                        Ok(receive_count) => {
                            if receive_count > 0 {
                                Ok(buffer[..receive_count].to_owned())
                            } else {
                                Err("Zero length datum received.".to_string())
                            }
                        },
                        Err(e) => Err(format!("Failed to read: {:?}", e)),
                    }
                },
                Err(e) => Err("Could not send message to mock AVR850".to_string()),
            }
        },
        Err(e) => Err("Could not connect to mock AVR850.".to_string()),
    }
}

#[test]
fn amx_value() {
    match connect_mock_avr850_send_and_receive("AMX".as_bytes()) {
        Ok(buffer) => assert_eq!(
            from_utf8(&buffer).unwrap().trim(),
            "AMXB<Device-SDKClass=Receiver><Device-Make=ARCAM><Device-Model=AVR850><Device-Revision=2.0.0>"),
        Err(e) => assert!(false, e),
    };
}

#[test]
fn get_default_brightness() {
    match connect_mock_avr850_send_and_receive(
        &create_request(ZoneNumber::One, Command::DisplayBrightness, &mut [0xf0]).unwrap()
    ) {
        Ok(buffer) => assert_eq!(
            parse_response(&buffer).unwrap(),
            (ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![1], 7)),
        Err(e) => assert!(false, e),
    };
}
