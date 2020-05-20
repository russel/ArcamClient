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

use log::debug;
use env_logger;

use num_traits::FromPrimitive;

use arcamclient::arcam_protocol::{
    AnswerCode, Brightness, Command, MuteState, PowerState, RC5Command, Request, Response, Source, VideoSource, ZoneNumber,
    PACKET_START, REQUEST_QUERY,
};

/// Zone state for an AVR. An AVR comprises a number of zones.
#[derive(Debug)]
struct ZoneState {
    power: Cell<PowerState>, // TODO Must Zone 1 be on for the Ethernet to work?
    volume: Cell<u8>, // Must be in range 0..100
    mute: Cell<MuteState>,
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
            brightness: Cell::new(Brightness::Level2),
        };
        amp_state.zones.insert(
            ZoneNumber::One,
            ZoneState{
                power: Cell::new(PowerState::On), // TODO Must Zone 1 be on for the Ethernet connection to work?
                volume: Cell::new(30),
                mute: Cell::new(MuteState::NotMuted),
                source: Cell::new(Source::CD),
            });
        amp_state.zones.insert(
            ZoneNumber::Two,
            ZoneState{
                power: Cell::new(PowerState::Standby),
                volume: Cell::new(20),
                mute: Cell::new(MuteState::NotMuted),
                source: Cell::new(Source::FollowZone1),
            });
        amp_state
    }
}

/// Return a response to a given request updating the state of the mock amp as needed.
fn create_command_response(request: &Request, amp_state: &mut AmpState) -> Result<Response, String>{
    match request.cc {
        Command::Power => {
            assert_eq!(request.data.len(), 1);
            if request.data[0] != REQUEST_QUERY {
                Err(format!("Incorrect Power command {:?}.", request.data[0]))
            } else {
                Ok(Response::new(request.zone, request.cc, AnswerCode::StatusUpdate, vec![amp_state.zones[&request.zone].power.get() as u8]).unwrap())
            }
        },
        Command::DisplayBrightness => {
            assert_eq!(request.data.len(), 1);
            if request.data[0] != REQUEST_QUERY {
                Err(format!("Incorrect DisplayBrightness command {:?}.", request.data[0]))
            } else {
                Ok(Response::new(request.zone, request.cc, AnswerCode::StatusUpdate, vec![amp_state.brightness.get() as u8]).unwrap())
            }
        },
        Command::SetRequestVolume => {
            assert_eq!(request.data.len(), 1);
            if request.data[0] == REQUEST_QUERY {
                Ok(Response::new(request.zone, request.cc, AnswerCode::StatusUpdate, vec![amp_state.zones[&request.zone].volume.get()]).unwrap())
            } else if request.data[0] < 100 {
                amp_state.zones[&request.zone].volume.set(request.data[0]);
                Ok(Response::new(request.zone, request.cc, AnswerCode::StatusUpdate, vec![amp_state.zones[&request.zone].volume.get()]).unwrap())
            } else {
                Err(format!("Failed to deal with SetRequestVolume command {:?}.", request.cc))
            }
        },
        Command::RequestCurrentSource => {
            assert_eq!(request.data.len(), 1);
            if request.data[0] != REQUEST_QUERY {
                Err(format!("Incorrect RequestCurrentSource command {:?}.", request.data[0]))
            } else {
                Ok(Response::new(request.zone, request.cc, AnswerCode::StatusUpdate, vec![amp_state.zones[&request.zone].source.get() as u8]).unwrap())
            }
        },
        Command::RequestMuteStatus => {
            assert_eq!(request.data.len(), 1);
            if request.data[0] != REQUEST_QUERY {
                Err(format!("Incorrect RequestMuteStatus command {:?}.", request.data[0]))
            } else {
                let is_mute = amp_state.zones[&request.zone].mute.get() as u8;
                Ok(Response::new(request.zone, request.cc, AnswerCode::StatusUpdate, vec![is_mute]).unwrap())
            }
        },
        Command::VideoSelection => {
            // TODO is the source the same as the video source in the amp?
            assert_eq!(request.data.len(), 1);
            if request.data[0] == REQUEST_QUERY {
                let video_source = match amp_state.zones[&request.zone].source.get() {
                    Source::BD => VideoSource::BD,
                    Source::SAT =>VideoSource::SAT,
                    Source::AV =>VideoSource::AV,
                    Source::PVR =>VideoSource::PVR,
                    Source::VCR =>VideoSource::VCR,
                    Source::GAME =>VideoSource::Game,
                    Source::STB =>VideoSource::STB,
                    _ => panic!("Illegal video source."),
                };
                Ok(Response::new(request.zone, request.cc, AnswerCode::StatusUpdate, vec![video_source as u8]).unwrap())
            } else {
                let source = match FromPrimitive::from_u8(request.data[0]).unwrap() {
                    VideoSource::BD => Source::BD,
                    VideoSource::SAT =>Source::SAT,
                    VideoSource::AV =>Source::AV,
                    VideoSource::PVR =>Source::PVR,
                    VideoSource::VCR =>Source::VCR,
                    VideoSource::Game =>Source::GAME,
                    VideoSource::STB =>Source::STB,
                };
                amp_state.zones[&request.zone].source.set(source);
                Ok(Response::new(request.zone, request.cc, AnswerCode::StatusUpdate, request.data.clone()).unwrap())
            }
        },
        Command::SimulateRC5IRCommand => {
            assert_eq!(request.data.len(), 2);
            let rc5command: RC5Command = (request.data[0], request.data[1]).into();
            match rc5command {
                RC5Command::DisplayOff => amp_state.brightness.set(Brightness::Off),
                RC5Command::DisplayL1 => amp_state.brightness.set(Brightness::Level1),
                RC5Command::DisplayL2 => amp_state.brightness.set(Brightness::Level2),
                RC5Command::MuteOn => {
                    assert_eq!(request.zone, ZoneNumber::One);
                    amp_state.zones[&request.zone].mute.set(MuteState::Muted);
                },
                RC5Command::MuteOff => {
                    assert_eq!(request.zone, ZoneNumber::One);
                    amp_state.zones[&request.zone].mute.set(MuteState::NotMuted);
                },
                RC5Command::Radio => amp_state.zones[&request.zone].source.set(Source::TUNER),
                RC5Command::CD => amp_state.zones[&request.zone].source.set(Source::CD),
                RC5Command::BD => amp_state.zones[&request.zone].source.set(Source::BD),
                RC5Command::AV => amp_state.zones[&request.zone].source.set(Source::AV) ,
                RC5Command::Sat => amp_state.zones[&request.zone].source.set(Source::SAT),
                RC5Command::PVR => amp_state.zones[&request.zone].source.set(Source::PVR),
                RC5Command::VCR => amp_state.zones[&request.zone].source.set(Source::VCR),
                RC5Command::Aux => amp_state.zones[&request.zone].source.set(Source::AUX),
                RC5Command::Display => amp_state.zones[&request.zone].source.set(Source::DISPLAY),
                RC5Command::Net => amp_state.zones[&request.zone].source.set(Source::NET),
                RC5Command::USB => amp_state.zones[&request.zone].source.set(Source::USB),
                RC5Command::STB  => amp_state.zones[&request.zone].source.set(Source::STB),
                RC5Command::Game => amp_state.zones[&request.zone].source.set(Source::GAME),
                RC5Command::PowerOn => {
                    assert_eq!(request.zone, ZoneNumber::One);
                    amp_state.zones[&request.zone].power.set(PowerState::On);
                },
                RC5Command::PowerOff => {
                    assert_eq!(request.zone, ZoneNumber::One);
                    amp_state.zones[&request.zone].power.set(PowerState::Standby);
                },
                RC5Command::Zone2PowerOn => {
                    assert_eq!(request.zone, ZoneNumber::Two);
                    amp_state.zones[&request.zone].power.set(PowerState::On)
                },
                RC5Command::Zone2PowerOff => {
                    assert_eq!(request.zone, ZoneNumber::Two);
                    amp_state.zones[&request.zone].power.set(PowerState::Standby)
                },
                RC5Command::Zone2MuteOn => {
                    assert_eq!(request.zone, ZoneNumber::Two);
                    amp_state.zones[&request.zone].mute.set(MuteState::Muted);
                },
                RC5Command::Zone2MuteOff => {
                    assert_eq!(request.zone, ZoneNumber::Two);
                    amp_state.zones[&request.zone].mute.set(MuteState::NotMuted);
                },
                _ => return Err("Not implemented.".to_string()),
            };
            Ok(Response::new(request.zone, request.cc, AnswerCode::StatusUpdate, request.data.clone()).unwrap())
        },
        x => Err(format!("Failed to deal with command {:?}.", x)),
    }
}

/// Handle a connection from a remote client.
fn handle_client(stream: &mut TcpStream, amp_state: &mut AmpState) {
    debug!("handle_client: got a connection from {}", stream.peer_addr().unwrap());
    loop {
        let mut buffer = [0; 256];
        match stream.read(&mut buffer) {
            Ok(count) => {
                if count > 0 {
                    let mut data = &buffer[..count];
                    debug!("handle_client: got a message {:?}", data);
                    // TODO Assume each is a complete packet and only one packet.
                    //   This may not be a good assumption even for the integration testing.
                    // Remove the output so as to speed up processing which then gets multiple packets to the client very quickly.
                    if data[0] == PACKET_START {
                        debug!("handle_client: processing Arcam request {:?}", data);
                        // TODO  How to deal with a buffer that has multiple packets?
                        loop {
                            match Request::parse_bytes(data) {
                                Ok((request, count)) => {
                                    data = &data[count..];
                                    debug!("handle_client: got a parse of {:?}, data left {:?}", &request, &data);
                                    debug!("handle_client: sending back {:?}", &create_command_response(&request, amp_state).unwrap());
                                    stream.write(&create_command_response(&request, amp_state).unwrap().to_bytes())
                                        .expect("handle_client: failed to write response");
                                },
                                Err(e) => {
                                    debug!("handle_client: failed to parse an apparent Arcam request: {:?}, {:?}", data, e);
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
                                        .expect("handle_client: failed to write AMX response");
                                } else {
                                    debug!("handle_client: unknown message, doing nothing.");
                                }
                            },
                            Err(e) => debug!("handle_client: buffer is not a string: {:?}", e),
                        }
                    }
                } else {
                    debug!("handle_client: no data read, assuming connection closed.");
                    break;
                }
            },
            Err(e) => {
                debug!("handle_client: read error: {:?}", e);
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
            debug!("create_default_amp_then_listen_on: server bound to {}", address);
            for stream in listener.incoming() {
                match stream {
                    Ok(mut s) => handle_client(&mut s, &mut amp_state),
                    Err(e) => debug!("create_default_amp_then_listen_on: failed to get incoming connection: {:?}", e),
                }
            }
            Ok(())
        },
        Err(e) => {
            debug!("create_default_amp_then_listen_on: failed to bind to {}: {:?}", address, e);
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
    env_logger::init();
    let args: Vec<String> = args().collect();
    debug!("main: args are {:?}", args);
    let default_port_number = 50000;
    let port_number = if args.len() > 1 { args[1].parse::<u16>().unwrap_or(default_port_number) } else { default_port_number };
    create_default_amp_then_listen_on(&SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_number))
}

#[cfg(test)]
mod tests {

    use super::{AmpState, create_command_response};

    use arcamclient::arcam_protocol::{
        AnswerCode, Brightness, Command, MuteState, PowerState, RC5Command, Request, Response, Source, ZoneNumber,
        REQUEST_QUERY,
        get_rc5command_data,
    };

    #[test]
    fn get_display_brightness() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.brightness.get(), Brightness::Level2);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![REQUEST_QUERY]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![Brightness::Level2 as u8]).unwrap());
        assert_eq!(amp_state.brightness.get(), Brightness::Level2);
    }

    #[test]
    fn set_display_brightness_error() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.brightness.get(), Brightness::Level2);
        match create_command_response(&Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![Brightness::Level2 as u8]).unwrap(), &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect DisplayBrightness command 2."),
        }
        assert_eq!(amp_state.brightness.get(), Brightness::Level2);
    }

    #[test]
    fn set_display_brightness_using_rc5() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.brightness.get(), Brightness::Level2);
        let rc5_data = get_rc5command_data(RC5Command::DisplayL1);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SimulateRC5IRCommand, data.clone()).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state.brightness.get(), Brightness::Level1);
    }

    #[test]
    fn get_zone_1_power() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].power.get(), PowerState::On);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::Power, vec![REQUEST_QUERY]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::Power, AnswerCode::StatusUpdate, vec![PowerState::On as u8]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].power.get(), PowerState::On);
    }

    #[test]
    fn set_zone_1_power_error() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].power.get(), PowerState::On);
        match create_command_response(&Request::new(ZoneNumber::One, Command::Power, vec![0x0]).unwrap(), &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect Power command 0."),
        }
    }

    #[test]
    fn set_zone_1_power_using_rc5() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].power.get(), PowerState::On);
        let rc5_data = get_rc5command_data(RC5Command::PowerOff);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SimulateRC5IRCommand, data.clone()).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].power.get(), PowerState::Standby);
    }

    #[test]
    fn get_zone_1_volume() {
        let mut amp_state: AmpState = Default::default();
        let volume = 30u8;
        assert_eq!(amp_state.zones[&ZoneNumber::One].volume.get(), volume);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![REQUEST_QUERY]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![volume]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].volume.get(), volume);
    }

    #[test]
    fn set_zone_1_volume() {
        let mut amp_state: AmpState = Default::default();
        let volume = 15u8;
        assert_eq!(amp_state.zones[&ZoneNumber::One].volume.get(), 30);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![volume]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![volume]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].volume.get(), volume);
    }

    #[test]
    fn get_zone_1_mute() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].mute.get(), MuteState::NotMuted);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::RequestMuteStatus, vec![REQUEST_QUERY]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::RequestMuteStatus, AnswerCode::StatusUpdate, vec![MuteState::NotMuted as u8]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].mute.get(), MuteState::NotMuted);
    }

    #[test]
    fn set_zone_1_mute_error() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].mute.get(), MuteState::NotMuted);
        match create_command_response(&Request::new(ZoneNumber::One, Command::RequestMuteStatus, vec![0x0]).unwrap(), &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect RequestMuteStatus command 0."),
        }
    }

    #[test]
    fn set_zone_1_mute_using_rc5() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].mute.get(), MuteState::NotMuted);
        let rc5_data = get_rc5command_data(RC5Command::MuteOn);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SimulateRC5IRCommand, data.clone()).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].mute.get(), MuteState::Muted);
    }

    #[test]
    fn get_zone_1_source() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].source.get(), Source::CD);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::CD as u8]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::One].source.get(), Source::CD);
    }

    #[test]
    fn set_zone_1_source_error() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].source.get(), Source::CD);
        match create_command_response(&Request::new(ZoneNumber::One, Command::RequestCurrentSource, vec![Source::TUNER as u8]).unwrap(), &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect RequestCurrentSource command 11."),
        }
    }

    #[test]
    fn set_zone_1_source_using_rc5() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::One].source.get(), Source::CD);
        let rc5_data = get_rc5command_data(RC5Command::BD);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SimulateRC5IRCommand, data.clone()).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap()
        );
        assert_eq!(amp_state.zones[&ZoneNumber::One].source.get(), Source::BD);
    }

    #[test]
    fn get_zone_2_power() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].power.get(), PowerState::Standby);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::Power, vec![REQUEST_QUERY]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::Two, Command::Power, AnswerCode::StatusUpdate, vec![PowerState::Standby as u8]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].power.get(), PowerState::Standby);
    }

    #[test]
    fn set_zone_2_power_error() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].power.get(), PowerState::Standby);
        match create_command_response(&Request::new(ZoneNumber::Two, Command::Power, vec![0x0]).unwrap(), &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect Power command 0."),
        }
    }

    #[test]
    fn set_zone_2_power_using_rc5() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].power.get(), PowerState::Standby);
        let rc5_data = get_rc5command_data(RC5Command::Zone2PowerOn);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, data.clone()).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].power.get(), PowerState::On);
    }

    #[test]
    fn get_zone_2_volume() {
        let mut amp_state: AmpState = Default::default();
        let volume = 20u8;
        assert_eq!(amp_state.zones[&ZoneNumber::Two].volume.get(), volume);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SetRequestVolume, vec![REQUEST_QUERY]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::Two, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![volume]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].volume.get(), volume);
    }

    #[test]
    fn set_zone_2_volume() {
        let mut amp_state: AmpState = Default::default();
        let volume = 15u8;
        assert_eq!(amp_state.zones[&ZoneNumber::Two].volume.get(), 20);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SetRequestVolume, vec![volume]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::Two, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![volume]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].volume.get(), volume);
    }

    #[test]
    fn get_zone_2_mute() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].mute.get(), MuteState::NotMuted);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::RequestMuteStatus, vec![REQUEST_QUERY]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::Two, Command::RequestMuteStatus, AnswerCode::StatusUpdate, vec![MuteState::NotMuted as u8]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].mute.get(), MuteState::NotMuted);
    }

    #[test]
    fn set_zone_2_mute_error() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].mute.get(), MuteState::NotMuted);
        match create_command_response(&Request::new(ZoneNumber::Two, Command::RequestMuteStatus, vec![0x1]).unwrap(), &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect RequestMuteStatus command 1."),
        }
    }

    #[test]
    fn set_zone_2_mute_using_rc5() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].mute.get(), MuteState::NotMuted);
        let rc5_data = get_rc5command_data(RC5Command::Zone2MuteOn);
        let data =vec! [rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, data.clone()).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].mute.get(), MuteState::Muted);
    }

    #[test]
    fn get_zone_2_source() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::Two, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::FollowZone1 as u8]).unwrap());
        assert_eq!(amp_state.zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
    }

    #[test]
    fn set_zone_2_source_error() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
        match create_command_response(&Request::new(ZoneNumber::Two, Command::RequestCurrentSource, vec![Source::TUNER as u8]).unwrap(), &mut amp_state) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect RequestCurrentSource command 11."),
        }
    }

    #[test]
    fn set_zone_2_source_using_rc5() {
        let mut amp_state: AmpState = Default::default();
        assert_eq!(amp_state.zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
        let rc5_data= get_rc5command_data(RC5Command::BD);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, data.clone()).unwrap(), &mut amp_state).unwrap(),
            Response::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap()
        );
        assert_eq!(amp_state.zones[&ZoneNumber::Two].source.get(), Source::BD);
    }

}
