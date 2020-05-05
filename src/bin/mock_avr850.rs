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

/*!
A program to simulate a AVR850 so that integration tests of the Arcam client can be undertaken.

The process opens port 50000 and listens for TCP packets using the Arcam IR remote control
protocol. Replies to queries must be sent within three seconds of the request being received. NB
This is an asynchronous question/answer system not a synchronous one.

When on a DAB radio such as Smooth, AVR850s send out
Command::RequestRDSDLSInformation response packets on a regular basis without
any prior request. So packets such as:

  [33, 1, 26, 0, 129, 12, 79, 110, 32, 65, 105, 114, 32, 78, 111, 119, 32, 111, 110, 32, 83, 109, 111, 111, 116, 104, 58, 32, 71, 97, 114, 121, 32, 75, 105, 110, 103, 0, 0, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32]

get sent out . They are always 129 long data packets containing a zero
terminates string. So in the above case:

 'O', 'n', ' ', 'A', 'i', 'r', ' ', 'N', 'o', 'w', ' ', 'o', 'n', ' ', 'S', 'm', 'o', 'o', 't', 'h', ':', ' ', 'G', 'a', 'r', 'y', ' ', 'K', 'i', 'n', 'g'
 "On Air Now on Smooth: Gary King."

also seen is the string:

  "Smooth - Your Relaxing Music Mix"

On a channel change some packets got emitted:

[33, 1, 24, 0, 16, 83, 109, 111, 111, 116, 104, 32, 67, 111, 117, 110, 116, 114, 121, 32, 32, 13]
[33, 1, 25, 0, 16, 67, 111, 117, 110, 116, 114, 121, 32, 77, 117, 115, 105, 99, 32, 32, 32, 13]
[33, 1, 26, 0, 129, 25, 78, 111, 119, 32, 111, 110, 32, 83, 109, 111, 111, 116, 104, 32, 67,
111, 117, 110, 116, 114, 121, 58, 32, 66, 114, 101, 116, 116, 32, 69, 108, 100, 114, 101, 100,
103, 101, 32, 119, 105, 116, 104, 32, 68, 114, 117, 110, 107, 32, 79, 110, 32, 89, 111, 117,
114, 32, 76, 111, 118, 101, 0, 0, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
32, 32, 13]

*/

use std::cell::Cell;
use std::collections::HashMap;
use std::env::args;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::str::from_utf8;

use arcamclient::arcam_protocol::{
    AnswerCode, Brightness, Command, Source, ZoneNumber,
    PACKET_START, REQUEST_VALUE,
    create_response, parse_request,
};

/// Zone state for an AVR. An AVR comprises a number of zones.
#[derive(Debug)]
struct ZoneState {
    volume: Cell<u8>,
    mute: Cell<bool>,
}

/// The state of a mock AVR.
#[derive(Debug)]
struct AmpState {
    zones: HashMap<ZoneNumber, ZoneState>,
    brightness: Cell<u8>,  // Values 0, 1, and 2 are the only ones allowed.
    source: Cell<Source>,
}

impl Default for AmpState {
    fn default() -> Self {
        let mut amp_state = Self {
            zones: HashMap::new(),
            brightness: Cell::new(1), // TODO Values 0, 1, and 2 are the only ones allowed.
            source: Cell::new(Source::TUNER),
        };
        amp_state.zones.insert(ZoneNumber::One, ZoneState{volume: Cell::new(30), mute: Cell::new(false)});
        amp_state.zones.insert(ZoneNumber::Two, ZoneState{volume: Cell::new(30), mute: Cell::new(true)});
        amp_state
    }
}

/// Return a response to a given request updating the state of the mock amp as needed.
fn create_command_response(zone: ZoneNumber, cc: Command, values: &[u8], amp_state: &mut AmpState) -> Result<Vec<u8>, String>{
    match cc {
        Command::DisplayBrightness =>
            if values[0] != REQUEST_VALUE {
                Err(format!("Incorrect DisplayBrightness request {:?}", values[0]))
            } else {
                Ok(create_response(zone, cc, AnswerCode::StatusUpdate, &[amp_state.brightness.get()]).unwrap())
            },
        Command::SetRequestVolume =>
            if values[0] == REQUEST_VALUE {
                Ok(create_response(zone, cc, AnswerCode::StatusUpdate, [amp_state.zones[&zone].volume.get()].as_ref()).unwrap())
            } else if values[0] < 100 {
                amp_state.zones[&zone].volume.set(values[0]);
                Ok(create_response(zone, cc, AnswerCode::StatusUpdate, [amp_state.zones[&zone].volume.get()].as_ref()).unwrap())
            } else {
                Err(format!("Failed to deal with SetRequestVolume command {:?}", cc))
            }
        //  TODO implement these two.
        Command::RequestCurrentSource =>
            if values[0] != REQUEST_VALUE {
                Err("Not implemented.".to_string())
            } else {
                Ok(create_response(zone, cc, AnswerCode::StatusUpdate, &[amp_state.source.get() as u8]).unwrap())
            },
        Command::VideoSelection => Err("Not implemented.".to_string()),
        _ => Err("Failed to deal with command.".to_string()),
    }
}

/// Handle a connection from a remote client.
fn handle_client(stream: &mut TcpStream, amp_state: &mut AmpState) {
    println!("####  mock_avr850: got a connection from {}", stream.peer_addr().unwrap());
    loop {
        let mut buffer = [0; 256];
        match stream.read(&mut buffer) {
            Ok(count) => {
                if count > 0 {
                    let data = &buffer[..count];
                    println!("####  mock_avr850: got a message {:?}", data);
                    // TODO Assume each is a complete packet and only one packet.
                    //   This may not be a good assumption even for the integration testing.
                    if data[0] == PACKET_START {
                        println!("####  mock_avr850: processing Arcam request {:?}", data);
                        match parse_request(data) {
                            Ok((zone, cc, values, count)) => {
                                stream.write(&create_command_response(zone, cc, &values, amp_state).unwrap())
                                    .expect("####  mock_avr850: failed to write response");
                            },
                            Err(e) => println!("####  mock_avr850: failed to parse an apparent Arcam request: {:?}, {:?}", data, e),
                        }
                    } else {
                        match from_utf8(data) {
                            Ok(s) => {
                                let message = s.trim();
                                if message == "AMX" {
                                    stream.write("AMXB<Device-SDKClass=Receiver><Device-Make=ARCAM><Device-Model=AVR850><Device-Revision=2.0.0>\r".as_bytes())
                                        .expect("####  mock_avr850: failed to write AMX response");
                                } else {
                                    println!("####  mock_avr850: unknown message, doing nothing.");
                                }
                            },
                            Err(e) => println!("####  mock_avr850: buffer is not a string: {:?}", e),
                        }
                    }
                } else {
                    println!("####  mock_avr850: no data read, assuming connection closed.");
                    break;
                }
            },
            Err(e) => {
                println!("####  mock_avr850: read error: {:?}", e);
                break;
            }
        }
    }
}

/// Create a mock amplifier and then listen for connections on the address provided.
///
/// Although a real AVR850 will only listen on port 50000, this simulator allows for any port to
/// support integration testing – tests may have to run faster than ports become available so
/// reusing the same port is not feasible.
fn create_default_amp_then_listen_on(address: &SocketAddr) -> Result<(), ()> {
    let mut amp_state: AmpState = Default::default();
    match TcpListener::bind(address) {
        Ok(listener) => {
            println!("####  mock_avr850: server bound to {}", address);
            for stream in listener.incoming() {
                match stream {
                    Ok(mut s) => handle_client(&mut s, &mut amp_state),
                    Err(e) => println!("####  mock_avr850: failed to get incoming connection: {:?}", e),
                }
            }
            Ok(())
        },
        Err(e) => {
            println!("####  mock_avr850: failed to bind to {}: {:?}", address, e);
            Err(())
        }
    }
}

/// Start the mock AVR850.
///
/// A real AVR850 only listens on port 50000, but this mock is allowed to listen on any port
/// in order to support integration testing where using a single port number can lead to
/// problems as a socket may not be closed as fast as new mocks are created. Testing must
/// avoid "Unable to bind socket: Address already in use".
fn main() -> Result<(), ()>{
    let args: Vec<String> = args().collect();
    println!("####  mock_avr850: args are {:?}", args);
    let default_port_number = 50000;
    let port_number = if args.len() > 1 { args[1].parse::<u16>().unwrap_or(default_port_number) } else { default_port_number };
    create_default_amp_then_listen_on(&SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_number))
}

#[cfg(test)]
mod tests {

    use super::{AmpState, create_command_response};

    use arcamclient::arcam_protocol::{
        AnswerCode, Command, ZoneNumber,
        REQUEST_VALUE,
        create_response,
    };

    #[test]
    fn get_display_brightness() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.brightness.get(), 1);
        assert_eq!(
            create_command_response(ZoneNumber::One, Command::DisplayBrightness, &mut [REQUEST_VALUE], &mut amp_state).unwrap(),
            create_response(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, &[0x01]).unwrap());
        assert_eq!(amp_state.brightness.get(), 1);
    }

    #[test]
    fn set_zone_1_volume() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].volume.get(), 30);
        assert_eq!(
            create_command_response(ZoneNumber::One, Command::SetRequestVolume, &mut [0x0f], &mut amp_state).unwrap(),
            create_response(ZoneNumber::One, Command::SetRequestVolume, AnswerCode::StatusUpdate, &[0x0f]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].volume.get(), 15);
    }
}
