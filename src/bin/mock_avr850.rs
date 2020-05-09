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
    AnswerCode, Brightness, Command, RC5Command, Source, ZoneNumber,
    PACKET_START, REQUEST_VALUE,
    create_response, get_rc5command_data, parse_request,
};

/// Zone state for an AVR. An AVR comprises a number of zones.
#[derive(Debug)]
struct ZoneState {
    power: Cell<bool>, // Zone 1 is always on but Zone 2 can be on or off.
    volume: Cell<u8>,
    mute: Cell<bool>,
    source: Cell<Source>,
}

/// The state of a mock AVR.
#[derive(Debug)]
struct AmpState {
    zones: HashMap<ZoneNumber, ZoneState>,
    brightness: Cell<Brightness>,
}

impl Default for AmpState {
    fn default() -> Self {
        let mut amp_state = Self {
            zones: HashMap::new(),
            brightness: Cell::new(Brightness::Level1),
        };
        amp_state.zones.insert(
            ZoneNumber::One,
            ZoneState{
                power: Cell::new(true),
                volume: Cell::new(30),
                mute: Cell::new(false),
                source: Cell::new(Source::CD),
            });
        amp_state.zones.insert(
            ZoneNumber::Two,
            ZoneState{
                power: Cell::new(false),
                volume: Cell::new(20),
                mute: Cell::new(true),
                source: Cell::new(Source::FollowZone1),
            });
        amp_state
    }
}

/// Return a response to a given request updating the state of the mock amp as needed.
fn create_command_response(zone: ZoneNumber, cc: Command, values: &[u8], amp_state: &mut AmpState) -> Result<Vec<u8>, String>{
    match cc {
        Command::DisplayBrightness => {
            assert_eq!(values.len(), 1);
            if values[0] != REQUEST_VALUE {
                Err(format!("Incorrect DisplayBrightness request {:?}", values[0]))
            } else {
                Ok(create_response(zone, cc, AnswerCode::StatusUpdate, &[amp_state.brightness.get() as u8]).unwrap())
            }
        },
        Command::SetRequestVolume => {
            assert_eq!(values.len(), 1);
            if values[0] == REQUEST_VALUE {
                Ok(create_response(zone, cc, AnswerCode::StatusUpdate, [amp_state.zones[&zone].volume.get()].as_ref()).unwrap())
            } else if values[0] < 100 {
                amp_state.zones[&zone].volume.set(values[0]);
                Ok(create_response(zone, cc, AnswerCode::StatusUpdate, [amp_state.zones[&zone].volume.get()].as_ref()).unwrap())
            } else {
                Err(format!("Failed to deal with SetRequestVolume command {:?}", cc))
            }
        },
        Command::RequestCurrentSource => {
            assert_eq!(values.len(), 1);
            if values[0] != REQUEST_VALUE {
                Err("Not implemented.".to_string())
            } else {
                Ok(create_response(zone, cc, AnswerCode::StatusUpdate, &[amp_state.zones[&zone].source.get() as u8]).unwrap())
            }
        },
        Command::RequestMuteStatus => {
            assert_eq!(values.len(), 1);
            if values[0] != REQUEST_VALUE {
                Err("Not implemented.".to_string())
            } else {
                let is_mute = if amp_state.zones[&zone].mute.get() { 0u8 } else { 1u8 };
                Ok(create_response(zone, cc, AnswerCode::StatusUpdate, &[is_mute]).unwrap())
            }
        },
        Command::SimulateRC5IRCommand => {
            assert_eq!(values.len(), 2);
            let rc5command: RC5Command = (values[0], values[1]).into();
            match rc5command {
                RC5Command::DisplayOff => {
                    amp_state.brightness.set(Brightness::Off);
                    Ok(create_response(zone, cc, AnswerCode::StatusUpdate, values).unwrap())
                },
                RC5Command::DisplayL1 => {
                    amp_state.brightness.set(Brightness::Level1);
                    Ok(create_response(zone, cc, AnswerCode::StatusUpdate, values).unwrap())
                },
                RC5Command::DisplayL2 => {
                    amp_state.brightness.set(Brightness::Level2);
                    Ok(create_response(zone, cc, AnswerCode::StatusUpdate, values).unwrap())
                },
                _ => Err("Not implemented.".to_string())
            }
        },
        x => Err(format!("Failed to deal with command {:?}", x)),
    }
}

/// Handle a connection from a remote client.
fn handle_client(stream: &mut TcpStream, amp_state: &mut AmpState) {
    eprintln!("mock_avr850: got a connection from {}", stream.peer_addr().unwrap());
    loop {
        let mut buffer = [0; 256];
        match stream.read(&mut buffer) {
            Ok(count) => {
                if count > 0 {
                    let mut data = &buffer[..count];
                    eprintln!("mock_avr850: got a message {:?}", data);
                    // TODO Assume each is a complete packet and only one packet.
                    //   This may not be a good assumption even for the integration testing.
                    // Remove the output so as to speed up processing which then gets multiple packets to the client very quickly.
                    if data[0] == PACKET_START {
                        eprintln!("mock_avr850: processing Arcam request {:?}", data);
                        // TODO  How to deal with a buffer that has multiple packets?
                        loop {
                            match parse_request(data) {
                                Ok((zone, cc, values, count)) => {
                                    data = &data[count..];
                                    eprintln!("mock_avr850: got a parse of ({:?}, {:?}, {:?}), data left {:?}", zone, cc, values, &data);
                                    eprintln!("mock_avr850: sending back {:?}", &create_command_response(zone, cc, &values, amp_state).unwrap());
                                    stream.write(&create_command_response(zone, cc, &values, amp_state).unwrap())
                                        .expect("mock_avr850: failed to write response");
                                },
                                Err(e) => {
                                    eprintln!("mock_avr850: failed to parse an apparent Arcam request: {:?}, {:?}", data, e);
                                    break;
                                },
                            }
                        }
                    } else {
                        match from_utf8(data) {
                            Ok(s) => {
                                let message = s.trim();
                                if message == "AMX" {
                                    stream.write("AMXB<Device-SDKClass=Receiver><Device-Make=ARCAM><Device-Model=AVR850><Device-Revision=2.0.0>\r".as_bytes())
                                        .expect("mock_avr850: failed to write AMX response");
                                } else {
                                    println!("mock_avr850: unknown message, doing nothing.");
                                }
                            },
                            Err(e) => println!("mock_avr850: buffer is not a string: {:?}", e),
                        }
                    }
                } else {
                    println!("mock_avr850: no data read, assuming connection closed.");
                    break;
                }
            },
            Err(e) => {
                println!("mock_avr850: read error: {:?}", e);
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
            println!("mock_avr850: server bound to {}", address);
            for stream in listener.incoming() {
                match stream {
                    Ok(mut s) => handle_client(&mut s, &mut amp_state),
                    Err(e) => println!("mock_avr850: failed to get incoming connection: {:?}", e),
                }
            }
            Ok(())
        },
        Err(e) => {
            println!("mock_avr850: failed to bind to {}: {:?}", address, e);
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
    println!("mock_avr850: args are {:?}", args);
    let default_port_number = 50000;
    let port_number = if args.len() > 1 { args[1].parse::<u16>().unwrap_or(default_port_number) } else { default_port_number };
    create_default_amp_then_listen_on(&SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_number))
}

#[cfg(test)]
mod tests {

    use super::{AmpState, create_command_response};

    use arcamclient::arcam_protocol::{
        AnswerCode, Brightness, Command, RC5Command, Source, ZoneNumber,
        REQUEST_VALUE,
        create_response, get_rc5command_data,
    };

    #[test]
    fn get_display_brightness() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.brightness.get(), Brightness::Level1);
        assert_eq!(
            create_command_response(ZoneNumber::One, Command::DisplayBrightness, &mut [REQUEST_VALUE], &mut amp_state).unwrap(),
            create_response(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, &[0x01]).unwrap());
        assert_eq!(amp_state.brightness.get(), Brightness::Level1);
    }

    #[test]
    fn set_display_brightness_error() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.brightness.get(), Brightness::Level1);
        match create_command_response(ZoneNumber::One, Command::DisplayBrightness, &mut [0x01], &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect DisplayBrightness request 1"),
        }
        assert_eq!(amp_state.brightness.get(), Brightness::Level1);
    }

    #[test]
    fn set_display_brightness() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.brightness.get(), Brightness::Level1);
        let (rc5_data_1, rc5_data_2) = get_rc5command_data(RC5Command::DisplayL2);
        assert_eq!(
            create_command_response(ZoneNumber::One, Command::SimulateRC5IRCommand, &mut [rc5_data_1, rc5_data_2], &mut amp_state).unwrap(),
            create_response(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, &[rc5_data_1, rc5_data_2]).unwrap());
        assert_eq!(amp_state.brightness.get(), Brightness::Level2);
    }

    #[test]
    fn get_zone_1_volume() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].volume.get(), 30);
        assert_eq!(
            create_command_response(ZoneNumber::One, Command::SetRequestVolume, &mut [REQUEST_VALUE], &mut amp_state).unwrap(),
            create_response(ZoneNumber::One, Command::SetRequestVolume, AnswerCode::StatusUpdate, &[0x1e]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].volume.get(), 30);
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

    #[test]
    fn get_zone_2_volume() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].volume.get(), 20);
        assert_eq!(
            create_command_response(ZoneNumber::Two, Command::SetRequestVolume, &mut [REQUEST_VALUE], &mut amp_state).unwrap(),
            create_response(ZoneNumber::Two, Command::SetRequestVolume, AnswerCode::StatusUpdate, &[0x14]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].volume.get(), 20);
    }

    #[test]
    fn set_zone_2_volume() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].volume.get(), 20);
        assert_eq!(
            create_command_response(ZoneNumber::Two, Command::SetRequestVolume, &mut [0x0f], &mut amp_state).unwrap(),
            create_response(ZoneNumber::Two, Command::SetRequestVolume, AnswerCode::StatusUpdate, &[0x0f]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].volume.get(), 15);
    }

    #[test]
    fn get_zone_1_mute_state() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].mute.get(), false);
        assert_eq!(
            create_command_response(ZoneNumber::One, Command::RequestMuteStatus, &mut [REQUEST_VALUE], &mut amp_state).unwrap(),
            create_response(ZoneNumber::One, Command::RequestMuteStatus, AnswerCode::StatusUpdate, &[0x1]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].mute.get(), false);
    }

    #[test]
    fn set_zone_1_mute_state() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].mute.get(), false);
        match create_command_response(ZoneNumber::One, Command::RequestMuteStatus, &mut [0x0], &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Not implemented."),
        }
    }

   #[test]
    fn get_zone_2_mute_state() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].mute.get(), true);
        assert_eq!(
            create_command_response(ZoneNumber::Two, Command::RequestMuteStatus, &mut [REQUEST_VALUE], &mut amp_state).unwrap(),
            create_response(ZoneNumber::Two, Command::RequestMuteStatus, AnswerCode::StatusUpdate, &[0x0]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].mute.get(), true);
    }

    #[test]
    fn set_zone_2_mute_state() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].mute.get(), true);
        match create_command_response(ZoneNumber::Two, Command::RequestMuteStatus, &mut [0x1], &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Not implemented."),
        }
    }

    #[test]
    fn get_zone_1_source() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].source.get(), Source::CD);
        assert_eq!(
            create_command_response(ZoneNumber::One, Command::RequestMuteStatus, &mut [REQUEST_VALUE], &mut amp_state).unwrap(),
            create_response(ZoneNumber::One, Command::RequestMuteStatus, AnswerCode::StatusUpdate, &[Source::CD as u8]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].source.get(), Source::CD);
    }

    #[test]
    fn set_zone_1_source() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].source.get(), Source::CD);
        match create_command_response(ZoneNumber::One, Command::RequestMuteStatus, &mut [Source::TUNER as u8], &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Not implemented."),
        }
    }

    #[test]
    fn get_zone_2_source() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
        assert_eq!(
            create_command_response(ZoneNumber::Two, Command::RequestCurrentSource, &mut [REQUEST_VALUE], &mut amp_state).unwrap(),
            create_response(ZoneNumber::Two, Command::RequestCurrentSource, AnswerCode::StatusUpdate, &[Source::FollowZone1 as u8]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
    }

    #[test]
    fn set_zone_2_source() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
        match create_command_response(ZoneNumber::Two, Command::RequestCurrentSource, &mut [Source::TUNER as u8], &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Not implemented."),
        }
    }


}
