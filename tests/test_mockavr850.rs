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

// Need to start a mock AVR850.
mod start_avr850;

use std::io::{Write, Read};
use std::net::{SocketAddr, TcpStream};
use std::str::from_utf8;

use arcamclient::arcam_protocol::{ZoneNumber, Command, AnswerCode, REQUEST_VALUE, create_request, parse_response, Source};

fn connect_to_mock_avr850() -> Result<TcpStream, String> {
    match TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], unsafe { start_avr850::PORT_NUMBER }))) {
        Ok(stream) => Ok(stream),
        Err(e) => Err(format!("Could not connect to mock AVR850: {:?}", e)),
    }
}

fn connect_mock_avr850_send_and_receive(send_data: &[u8]) -> Result<Vec<u8>, String> {
    match connect_to_mock_avr850() {
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
                Err(e) => Err(format!("Could not send message to mock AVR850: {:?}", e)),
            }
        },
        Err(e) => Err(format!("Could not connect to mock AVR850: {:?}", e)),
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
        &create_request(ZoneNumber::One, Command::DisplayBrightness, &mut [REQUEST_VALUE]).unwrap()
    ) {
        Ok(buffer) => assert_eq!(
            parse_response(&buffer).unwrap(),
            (ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![1], 7)),
        Err(e) => assert!(false, e),
    };
}

#[test]
fn send_multi_packet_message() {
     let mut send_data = create_request(ZoneNumber::One, Command::DisplayBrightness, &[REQUEST_VALUE]).unwrap();
    send_data.append(&mut create_request(ZoneNumber::One, Command::RequestCurrentSource, &[REQUEST_VALUE]).unwrap());
    send_data.append(&mut create_request(ZoneNumber::Two, Command::RequestCurrentSource, &[REQUEST_VALUE]).unwrap());
    match connect_to_mock_avr850() {
        Ok(mut stream) => {
            match stream.write(&send_data) {
                Ok(send_count) => {
                    assert_eq!(send_count, send_data.len());
                    let mut buffer = [0; 1024];
                    let mut response_count = 0;
                    match stream.read(&mut buffer) {
                        Ok(receive_count) => {
                            if receive_count > 0 {
                                let mut data = buffer[..receive_count].to_owned();
                                assert_eq!(
                                    parse_response(&data).unwrap(),
                                    (ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![0x01], 7)
                                );
                                response_count += 1;
                                if data.len() > 7 {
                                    for _ in 0..7 { data.remove(0); }
                                    assert_eq!(
                                        parse_response(&data).unwrap(),
                                        (ZoneNumber::One, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::TUNER as u8], 7)
                                    );
                                    response_count += 1;
                                    if data.len() > 7 {
                                        for _ in 0..7 { data.remove(0); }
                                        eprintln!("YYYYYY {:?}", data);
                                        assert_eq!(
                                            parse_response(&data).unwrap(),
                                            (ZoneNumber::Two, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::FollowZone1 as u8], 7)
                                        );
                                        response_count += 1;
                                    }
                                }
                            } else {
                                assert!(false, "Zero length datum received.")
                            }
                            assert!(response_count > 0);
                            if response_count < 3 {
                                match stream.read(&mut buffer) {
                                    Ok(receive_count) => {
                                        if receive_count > 0 {
                                            let mut data = buffer[..receive_count].to_owned();
                                            assert_eq!(
                                                parse_response(&data).unwrap(),
                                                (ZoneNumber::One, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::TUNER as u8], 7)
                                            );
                                            response_count += 1;
                                            if data.len() > 7 {
                                                for _ in 0..7 { data.remove(0); }
                                                eprintln!("YYYYYY {:?}", data);
                                                assert_eq!(
                                                    parse_response(&data).unwrap(),
                                                    (ZoneNumber::Two, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::FollowZone1 as u8], 7)
                                                );
                                                response_count += 1;
                                            }
                                        } else {
                                            assert!(false, "Zero length datum received.")
                                        }
                                    },
                                    Err(e) => assert!(false, "Failed to read: {:?}", e),
                                }
                            }
                            assert!(response_count > 1);
                            if response_count < 3 {
                                match stream.read(&mut buffer) {
                                    Ok(receive_count) => {
                                        if receive_count > 0 {
                                            let mut data = buffer[..receive_count].to_owned();
                                            assert_eq!(
                                                parse_response(&data).unwrap(),
                                                (ZoneNumber::Two, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::FollowZone1 as u8], 7)
                                            );
                                            response_count += 1;
                                        }
                                    },
                                    Err(e) => assert!(false, "Failed to read: {:?}", e),
                                };
                            }
                        },
                        Err(e) => assert!(false, "Failed to read: {:?}", e),
                    }
                    assert_eq!(response_count, 3);
                },
                Err(e) => assert!(false, "Could not send message to mock AVR850: {:?}", e),
            }
        },
        Err(e) => assert!(false, e),
    }
}
