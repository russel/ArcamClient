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

use arcamclient::arcam_protocol::{AnswerCode, Command, RC5Command, Request, Response, Source, ZoneNumber, REQUEST_QUERY, get_rc5command_data, Brightness};

fn connect_to_mock_avr850() -> TcpStream {
    match TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], unsafe { start_avr850::PORT_NUMBER }))) {
        Ok(stream) => stream,
        Err(e) => panic!("Could not connect to mock AVR850: {:?}", e),
    }
}

fn send_to_mock_avr850(mut stream: &TcpStream, data: &[u8]) {
    match stream.write(data) {
        Ok(count) => assert_eq!(count, data.len()),
        Err(e) => panic!("Failed to send data: {:?}", e),
    }
}

fn read_from_mock_avr850(mut stream: &TcpStream, buffer: &mut [u8]) -> usize {
    match stream.read(buffer) {
        Ok(count) => {
            if count == 0 {
                panic!("Zero length read.");
            };
            count
        },
        Err(e) => panic!("Failed to read data: {:?}", e),
    }
}

fn connect_mock_avr850_send_and_receive(send_data: &[u8]) -> Vec<u8> {
    let stream = connect_to_mock_avr850();
    send_to_mock_avr850(&stream, send_data);
    let mut buffer = [0u8; 4096];
    let count = read_from_mock_avr850(&stream, &mut buffer);
    buffer[..count].to_vec()
}

#[test]
fn amx_value() {
    let data = connect_mock_avr850_send_and_receive("AMX".as_bytes());
    assert_eq!(
        from_utf8(&data).unwrap().trim(),
        "AMXB<Device-SDKClass=Receiver><Device-Make=ARCAM><Device-Model=AVR850><Device-Revision=2.0.0>"
    );
}

#[test]
fn get_default_brightness() {
    let data = connect_mock_avr850_send_and_receive(
        &Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![REQUEST_QUERY]).unwrap().to_bytes()
    );
    assert_eq!(
        Response::parse_bytes(&data).unwrap(),
        (Response::new(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![Brightness::Level2 as u8]).unwrap(), 7)
    );
}

#[test]
fn set_zone_1_source_to_bd() {
    let rc5_data = get_rc5command_data(RC5Command::BD);
    let data = vec![rc5_data.0, rc5_data.1];
    let response_data = connect_mock_avr850_send_and_receive(
        &Request::new(ZoneNumber::One, Command::SimulateRC5IRCommand, data.clone()).unwrap().to_bytes()
    );
    assert_eq!(
        Response::parse_bytes(&response_data).unwrap(),
        (Response::new(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data.clone()).unwrap(), 8)
    );
}

#[test]
fn send_multi_packet_message() {
    // NB It is not certain that the order of responses to the requests matches the order
    // of the requests. So check only that the three expected responses arrived, do not
    // impose an order on the Responses.
    let mut send_data = Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![REQUEST_QUERY]).unwrap().to_bytes();
    send_data.append(&mut Request::new(ZoneNumber::One, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap().to_bytes());
    send_data.append(&mut Request::new(ZoneNumber::Two, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap().to_bytes());
    let stream = connect_to_mock_avr850();
    send_to_mock_avr850(&stream, &send_data);
    let mut buffer = [0u8; 4096];
    let mut responses: Vec<Response> = vec![];
    while responses.len() < 3 {
        let receive_count = read_from_mock_avr850(&stream, &mut buffer);
        let mut data = &buffer[..receive_count];
        while data.len() > 0 {
            match Response::parse_bytes(&data) {
                Ok((response, count)) => {
                    data = &data[count..];
                    responses.push(response);
                },
                Err(e) => assert!(false),
            }
        }
    }
    assert!(responses.contains(
        &Response::new(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![Brightness::Level2 as u8]).unwrap()
    ));
    assert!(responses.contains(
        &Response::new(ZoneNumber::One, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::BD as u8]).unwrap()
    ));
    assert!(responses.contains(
        &Response::new(ZoneNumber::Two, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::FollowZone1 as u8]).unwrap()
    ));
}
