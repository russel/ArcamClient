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

//! A program to simulate (aka mock) an AVR850 so that integration tests of the ArcamClient can
//! be undertaken.
//!
//! The process opens a port on localhost, by default 50000, and listens for connections.
//!
//! A real AVR850 accepts only a single connection at a time: if a new connection arrives it
//! closes the prior connection. This behaviour is not replicated by this mock. Instead many
//! connections are allowed but each connection has its own distinct amplifier state.  In effect
//! this process is a server of mock AVR850 processes.
//!
//! A real AVR850 only listens on port 50000.  This simulation/mock can be started listening on
//! any port in the range [50001, 65535]. If all processes started by the integration tests used
//! the same port then there is the possibility that a process is started whilst another is
//! running or the port was not yet ready for reuse. In both cases the later process would fail
//! to start, and so the integration test would fail due to lack of connection to a mock
//! AVR850. Testing must avoid "Unable to bind socket: Address already in use".
//!
//! TCP packets received by a (mock) AVR850 must either be AMX requests or requests that use the
//! Arcam packet protocol. Replies to queries are sent within three seconds of the request being
//! received. NB This is an asynchronous question/answer system not a synchronous
//! one. Experimentation indicates that a real AVR850 responds to anything other than an Arcam
//! packet with an AMX response. This behaviour is replicated by this simulation/mock.
//!
//! When on a DAB radio such as Smooth, an AVR850 sends out Command::DLSPDTInformation response
//! packets on a regular basis without any prior request. So packets such as:
//!
//!  [33, 1, 26, 0, 129, 12, 79, 110, 32, 65, 105, 114, 32, 78, 111, 119, 32, 111, 110, 32,
//!   83, 109, 111, 111, 116, 104, 58, 32, 71, 97, 114, 121, 32, 75, 105, 110, 103, 0, 0,
//!   32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
//!   32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
//!   32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
//!   32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
//!   32, 32, 32, 32, 32, 32, 32]
//!
//! get sent out. They are always 129 long data packets containing a nul terminated string –
//! double nul in this case, assumed single nul terminated for 128 character strings.  So in
//! this case:
//!
//!  'O', 'n', ' ', 'A', 'i', 'r', ' ', 'N', 'o', 'w', ' ', 'o', 'n', ' ', 'S', 'm', 'o', 'o', 't', 'h', ':', ' ', 'G', 'a', 'r', 'y', ' ', 'K', 'i', 'n', 'g'
//!
//! or
//!
//!  "On Air Now on Smooth: Gary King."
//!
//! This is different to what is stated in the documentation.  also seen is the string:
//!
//!  "Smooth - Your Relaxing Music Mix"
//!
//! On a channel change some packets got emitted:
//!
//! [33, 1, 24, 0, 16, 83, 109, 111, 111, 116, 104, 32, 67, 111, 117, 110, 116, 114, 121, 32, 32, 13]
//! [33, 1, 25, 0, 16, 67, 111, 117, 110, 116, 114, 121, 32, 77, 117, 115, 105, 99, 32, 32, 32, 13]
//! [33, 1, 26, 0, 129, 25, 78, 111, 119, 32, 111, 110, 32, 83, 109, 111, 111, 116, 104, 32, 67,
//! 111, 117, 110, 116, 114, 121, 58, 32, 66, 114, 101, 116, 116, 32, 69, 108, 100, 114, 101, 100,
//! 103, 101, 32, 119, 105, 116, 104, 32, 68, 114, 117, 110, 107, 32, 79, 110, 32, 89, 111, 117,
//! 114, 32, 76, 111, 118, 101, 0, 0, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
//! 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
//! 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
//! 32, 32, 13]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::env::args;
use std::rc::Rc;
use std::time::SystemTime;

use log::debug;
use env_logger;

use gio;
use gio::prelude::*;
use gio_futures::{SocketConnection, SocketListener};

use futures;
use futures::prelude::*;

use num_traits::FromPrimitive;

use arcamclient::arcam_protocol::{
    AnswerCode, Brightness, Command, MuteState, PowerState, RC5Command, Request, Response, Source, VideoSource, ZoneNumber,
    PACKET_START, REQUEST_QUERY,
};

/// State for an AVR zone.
#[derive(Debug)]
struct ZoneState {
    power: Cell<PowerState>,
    volume: Cell<u8>, // Must be in range [0..100)
    mute: Cell<MuteState>,
    source: Cell<Source>,
}

/// The state of a mock AVR.  An AVR850 comprises two zones, an AVR600 comprised 3 zones.
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
                power: Cell::new(PowerState::On),
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
fn create_command_response(request: &Request, amp_state_ptr: Rc<RefCell<AmpState>>, sender: Option<futures::channel::mpsc::Sender<Vec<u8>>>) -> Result<Response, String>{
    let amp_state = amp_state_ptr.borrow_mut();
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
                RC5Command::Radio => {
                    amp_state.zones[&request.zone].source.set(Source::TUNER);
                    if sender.is_some() {
                        glib::MainContext::default().spawn_local(
                            send_tuner_data_and_start_dls_pdt_sending(amp_state_ptr.clone(), request.zone, sender.unwrap().clone()));
                    }
                },
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
                RC5Command::SetZone2ToFollowZone1 => {
                    assert_eq!(request.zone, ZoneNumber::Two);
                    amp_state.zones[&request.zone].source.set(Source::FollowZone1)
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

/// When an AVR850 is using an FM or DAB tuner (aka radio) source, it sends out extra DLS/PDT
/// packets. These normally provide information about the show currently on the station and the
/// piece currently being played. Simulate this without even trying to be too realistic.
async fn send_tuner_data_and_start_dls_pdt_sending(amp_state_ptr: Rc<RefCell<AmpState>>, zone: ZoneNumber, mut sender: futures::channel::mpsc::Sender<Vec<u8>>) {
    // TODO What about FM as well as DAB?
    // Station name is always 16 bytes long.
    let station_name = "A DAB Station   ";
    assert_eq!(station_name.len(), 16);
    sender.send(Response::new(zone, Command::RequestDABStation, AnswerCode::StatusUpdate,
                              station_name.as_bytes().to_vec()).unwrap().to_bytes()).await.expect("Failed to send station name.");
    let programme_type = "Good Music      ";
    // Programme type is always 16 bytes long.
    assert_eq!(programme_type.len(), 16);
    sender.send(Response::new(zone, Command::ProgrammeTypeCategory, AnswerCode::StatusUpdate,
                              programme_type.as_bytes().to_vec()).unwrap().to_bytes()).await.expect("Failed to send programme type.");
    glib::timeout_add_seconds_local(4, {
        let mut s = sender.clone();
        move || {
            let zone_source = amp_state_ptr.borrow().zones[&zone].source.get(); //  AMP_STATE.lock().unwrap().zones[&zone].source.get();
            if zone_source == Source::TUNER || zone_source == Source::TUNERDAB {
                // DLS/PDT data is always 128 bytes long according to the manual, but experiment
                // indicates a real AVR850 returns 129 characters.The manual states that the
                // string is padded with spaces to fill the 128 characters. A real AVR850 seems
                // to null terminate the string, with two nulls if possible, and then pad the
                // 129 characters with spaces.
                let mut dls_pdt_buffer = [' ' as u8; 129];
                // Quite weird that elapsed doesn't return zero!
                let dsl_pdt_data = format!("This DLS/PDT information sent after {:?}", SystemTime::now().elapsed().unwrap());
                assert!(dsl_pdt_data.len() <= 128);
                let mut i = 0;
                for c in dsl_pdt_data.bytes() {
                    dls_pdt_buffer[i] = c;
                    i += 1;
                }
                assert_eq!(i, dsl_pdt_data.len());
                dls_pdt_buffer[i] = 0;
                // An AVR850 appears to put two null bytes in the buffer if it can.
                if dsl_pdt_data.len() < 128 {
                    i += 1;
                    dls_pdt_buffer[i] = 0;
                }
                debug!("send_tuner_rds_dls:  Sending {:?}", &dls_pdt_buffer.to_vec()); // Can only print an array of 32 or less items.
                s.try_send(Response::new(zone, Command::DLSPDTInformation, AnswerCode::StatusUpdate,
                                         dls_pdt_buffer.to_vec()).unwrap().to_bytes()).expect("Failed to send DLS/PDT.");
                Continue(true)
            } else {
                Continue(false)
            }
        }
    });
}

// Make it easier to display a InetSocketAddress in a human readable form for debugging.
fn create_string_for_inetsocketaddress(address: &gio::InetSocketAddress) -> String {
    format!("{}:{}", gio::InetAddressExt::to_string(&address.get_address().unwrap()), address.get_port())
}

// Make it easier to display a SocketAddress in a human readable form for debugging.
fn create_string_for_socketaddress(address: &gio::SocketAddress) -> String {
    create_string_for_inetsocketaddress(&address.clone().downcast::<gio::InetSocketAddress>().unwrap())
}

/// Handle a connection from a remote client.
///
/// Read [Request](struct.Request.html) byte sequences as they arrive, parse them to create
/// [Request](struct.Request.html)s and then send a [Response](struct.Response.html) as a real
/// AVR850 might.
///
/// Each connection has its own [AmpState](struct.AmpState.html) so appears as a distinct mock AVR850.
async fn process_connection(connection: SocketConnection, amp_state_ptr: Rc<RefCell<AmpState>>) {
    let remote_address = connection.get_remote_address().unwrap();
    debug!("process_connection: connection from {}", &create_string_for_socketaddress(&remote_address));
    let (mut reader, mut writer) = connection.split();
    let (mut tx_send_queue, mut rx_send_queue) = futures::channel::mpsc::channel::<Vec<u8>>(10);
    glib::MainContext::default().spawn_local(async move {
        while let Some(data) = rx_send_queue.next().await {
            match writer.write_all(&data).await {
                Ok(_) => { debug!("process_connection: successfully sent data {:?}", &data) },
                Err(e) => { debug!("process_connection: error sending data – {:?}", e) },
            }
        }
    });
    loop {
        let mut buffer = [0u8; 1024];  // Must be bigger than the byte size of the maximum number of simultaneous packets receivable.
        match reader.read(&mut buffer).await {
            Ok(read_count) => {
                if read_count == 0 {
                    debug!("process_connection: zero length read, assuming connection from {} closed.", &create_string_for_socketaddress(&remote_address));
                    break;
                } else {
                    let mut data = &buffer[..read_count];
                    if data[0] == PACKET_START {
                        // Process an Arcam packet. There may be more than one packet in this TCP message.
                        // TODO What happens if there is an AMX\r within the TCP message?
                        while data.len() > 0 {
                            match Request::parse_bytes(data) {
                                Ok((request, count)) => {
                                    data = &data[count..];
                                    match create_command_response(&request, amp_state_ptr.clone(), Some(tx_send_queue.clone())) {
                                        Ok(response) => {
                                            debug!("process_connection: sending the response {:?}", &response);
                                            match tx_send_queue.try_send(response.to_bytes()) {
                                                Ok(_) => debug!("process_connection: put response on the queue."),
                                                Err(e) => debug!("process_connection: failed to put response on the queue – {}", e),
                                            };
                                        },
                                        Err(e) => debug!("process_connection: failed to process a request – {}", e),
                                    };
                                },
                                Err(e) => debug!("process_connection: failed to parse {:?} as a request – {}", &data, e),
                            };
                        }
                    } else {
                        debug!("process_connection: received a non-packet message – {:?}", &data);
                        match tx_send_queue.try_send(
                            "AMXB<Device-SDKClass=Receiver><Device-Make=ARCAM><Device-Model=AVR850><Device-Revision=2.0.0>\r".as_bytes().to_vec()) {
                            Ok(_) => debug!("process_connection: put AMX response on the queue."),
                            Err(e) => debug!("process_connection: failed to put AMX response on the queue – {}", e),
                        }
                    }
                }
            },
            Err(e) => debug!("process_connection: read failed – {}", e),
        };
    }
    debug!("process_connection: terminating connection with {:?}", &create_string_for_socketaddress(&remote_address));
}

/// Listen on localhost:<port_number> for connections and process each one.
///
/// Although a real AVR850 listens only on port 50000, this simulator allows for any port to
/// support integration testing – tests may have to run faster than ports become available after
/// being closed, so reusing the same port is not feasible.
///
/// A real AVR850 only allows a single connection at a time, dropping the prior connection in
/// favour of the new connection request. This is not modelled in this mock. Instead each
/// connection looks like a connection to a different mock AVR850.
async fn run_connection_listener(port_number: u16) {
    let server = SocketListener::new();
    let address = gio::InetSocketAddress::new(&gio::InetAddress::from_string("127.0.0.1").unwrap(), port_number);
    server.add_address(&address, gio::SocketType::Stream, gio::SocketProtocol::Tcp, None::<&glib::Object>).expect("Failed to bind to address.");
    debug!("run_connection_listener: Listening on {}", &create_string_for_inetsocketaddress(&address));
    let mut incoming = server.incoming();
    while let Some(socket_connection) = incoming.next().await {
        match socket_connection {
            Ok(s_c) => {
                let local_address = match s_c.get_local_address() {
                    Ok(s_a) => create_string_for_socketaddress(&s_a),
                    Err(_) => "error".to_string(),
                };
                let remote_address = match s_c.get_remote_address() {
                    Ok(s_a) => create_string_for_socketaddress(&s_a),
                    Err(_) => "error".to_string(),
                };
                debug!("run_connection_listener: got a connection on {} from {}", &local_address, &remote_address);
                glib::MainContext::default().spawn_local(process_connection(s_c, Rc::new(RefCell::new(AmpState::default()))));
            },
            Err(e) => debug!("run_connection_listener: got an errorful connection request – {}", e),
        }

    }
    debug!("run_connection_listener: finished.");
}

/// Start the mock AVR850.
///
/// A real AVR850 listens only on port 50000, but this mock is allowed to listen on any port in
/// order to support integration testing where using a single port number can lead to problems
/// as a socket may not be available for use after being closed as fast as new mocks are
/// created. Testing must avoid "Unable to bind socket: Address already in use".
///
/// A real AVR850 only allows a single connection at a time, dropping the prior connection in
/// favour of the new connection request. This is not modelled in this mock. Instead each
/// connection looks like a connection to a different mock AVR850.
fn main() {
    env_logger::init();
    let args: Vec<String> = args().collect();
    debug!("main: args are {:?}", args);
    let default_port_number = 50000;
    let port_number = if args.len() > 1 { args[1].parse::<u16>().unwrap_or(default_port_number) } else { default_port_number };
    debug!("main: starting event loop.");
    let context = glib::MainContext::default();
    context.push_thread_default();
    context.block_on(run_connection_listener(port_number));
    context.pop_thread_default();
    debug!("main: event loop terminated.");
}

#[cfg(test)]
mod tests {

    use std::cell::RefCell;
    use std::rc::Rc;

    use super::{AmpState, create_command_response};

    use arcamclient::arcam_protocol::{
        AnswerCode, Brightness, Command, MuteState, PowerState, RC5Command, Request, Response, Source, ZoneNumber,
        REQUEST_QUERY,
        get_rc5command_data,
    };

    #[test]
    fn get_display_brightness() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().brightness.get(), Brightness::Level2);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![REQUEST_QUERY]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![Brightness::Level2 as u8]).unwrap());
        assert_eq!(amp_state_ptr.borrow().brightness.get(), Brightness::Level2);
    }

    #[test]
    fn set_display_brightness_error() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().brightness.get(), Brightness::Level2);
        match create_command_response(&Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![Brightness::Level2 as u8]).unwrap(), amp_state_ptr.clone(), None) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect DisplayBrightness command 2."),
        }
        assert_eq!(amp_state_ptr.borrow().brightness.get(), Brightness::Level2);
    }

    #[test]
    fn set_display_brightness_using_rc5() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().brightness.get(), Brightness::Level2);
        let rc5_data = get_rc5command_data(RC5Command::DisplayL1);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SimulateRC5IRCommand, data.clone()).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state_ptr.borrow().brightness.get(), Brightness::Level1);
    }

    #[test]
    fn get_zone_1_power() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].power.get(), PowerState::On);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::Power, vec![REQUEST_QUERY]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::Power, AnswerCode::StatusUpdate, vec![PowerState::On as u8]).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].power.get(), PowerState::On);
    }

    #[test]
    fn set_zone_1_power_error() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].power.get(), PowerState::On);
        match create_command_response(&Request::new(ZoneNumber::One, Command::Power, vec![0x0]).unwrap(), amp_state_ptr.clone(), None) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect Power command 0."),
        }
    }

    #[test]
    fn set_zone_1_power_using_rc5() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].power.get(), PowerState::On);
        let rc5_data = get_rc5command_data(RC5Command::PowerOff);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SimulateRC5IRCommand, data.clone()).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].power.get(), PowerState::Standby);
    }

    #[test]
    fn get_zone_1_volume() {
        let volume = 30u8;
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].volume.get(), volume);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![REQUEST_QUERY]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![volume]).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].volume.get(), volume);
    }

    #[test]
    fn set_zone_1_volume() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].volume.get(), 30);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![0x0f]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![0x0f]).unwrap()
        );
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].volume.get(), 15);
    }

    #[test]
    fn get_zone_1_mute() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].mute.get(), MuteState::NotMuted);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::RequestMuteStatus, vec![REQUEST_QUERY]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::RequestMuteStatus, AnswerCode::StatusUpdate, vec![MuteState::NotMuted as u8]).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].mute.get(), MuteState::NotMuted);
    }

    #[test]
    fn set_zone_1_mute_error() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].mute.get(), MuteState::NotMuted);
        match create_command_response(&Request::new(ZoneNumber::One, Command::RequestMuteStatus, vec![0x0]).unwrap(), amp_state_ptr.clone(), None) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect RequestMuteStatus command 0."),
        }
    }

    #[test]
    fn set_zone_1_mute_using_rc5() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].mute.get(), MuteState::NotMuted);
        let rc5_data = get_rc5command_data(RC5Command::MuteOn);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SimulateRC5IRCommand, data.clone()).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].mute.get(), MuteState::Muted);
    }

    #[test]
    fn get_zone_1_source() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].source.get(), Source::CD);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::CD as u8]).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].source.get(), Source::CD);
    }

    #[test]
    fn set_zone_1_source_error() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].source.get(), Source::CD);
        match create_command_response(&Request::new(ZoneNumber::One, Command::RequestCurrentSource, vec![Source::TUNER as u8]).unwrap(), amp_state_ptr.clone(), None) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect RequestCurrentSource command 11."),
        }
    }

    #[test]
    fn set_zone_1_source_using_rc5() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].source.get(), Source::CD);
        let rc5_data = get_rc5command_data(RC5Command::BD);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::One, Command::SimulateRC5IRCommand, data.clone()).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::One, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap()
        );
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::One].source.get(), Source::BD);
    }

    #[test]
    fn get_zone_2_power() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].power.get(), PowerState::Standby);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::Power, vec![REQUEST_QUERY]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::Two, Command::Power, AnswerCode::StatusUpdate, vec![PowerState::Standby as u8]).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].power.get(), PowerState::Standby);
    }

    #[test]
    fn set_zone_2_power_error() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].power.get(), PowerState::Standby);
        match create_command_response(&Request::new(ZoneNumber::Two, Command::Power, vec![0x0]).unwrap(), amp_state_ptr.clone(), None) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect Power command 0."),
        }
    }

    #[test]
    fn set_zone_2_power_using_rc5() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].power.get(), PowerState::Standby);
        let rc5_data = get_rc5command_data(RC5Command::Zone2PowerOn);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, data.clone()).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].power.get(), PowerState::On);
    }

    #[test]
    fn get_zone_2_volume() {
        let volume = 20u8;
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].volume.get(), volume);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SetRequestVolume, vec![REQUEST_QUERY]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::Two, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![volume]).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].volume.get(), volume);
    }

    #[test]
    fn set_zone_2_volume() {
        let volume = 15u8;
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].volume.get(), 20);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SetRequestVolume, vec![volume]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::Two, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![volume]).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].volume.get(), volume);
    }

    #[test]
    fn get_zone_2_mute() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].mute.get(), MuteState::NotMuted);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::RequestMuteStatus, vec![REQUEST_QUERY]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::Two, Command::RequestMuteStatus, AnswerCode::StatusUpdate, vec![MuteState::NotMuted as u8]).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].mute.get(), MuteState::NotMuted);
    }

    #[test]
    fn set_zone_2_mute_error() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].mute.get(), MuteState::NotMuted);
        match create_command_response(&Request::new(ZoneNumber::Two, Command::RequestMuteStatus, vec![0x1]).unwrap(), amp_state_ptr.clone(), None) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect RequestMuteStatus command 1."),
        }
    }

    #[test]
    fn set_zone_2_mute_using_rc5() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].mute.get(), MuteState::NotMuted);
        let rc5_data = get_rc5command_data(RC5Command::Zone2MuteOn);
        let data =vec! [rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, data.clone()).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].mute.get(), MuteState::Muted);
    }

    #[test]
    fn get_zone_2_source() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::Two, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::FollowZone1 as u8]).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
    }

    #[test]
    fn set_zone_2_source_error() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
        match create_command_response(&Request::new(ZoneNumber::Two, Command::RequestCurrentSource, vec![Source::TUNER as u8]).unwrap(), amp_state_ptr.clone(), None) {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, "Incorrect RequestCurrentSource command 11."),
        }
    }

    #[test]
    fn set_zone_2_source_using_rc5() {
        let amp_state_ptr = Rc::new(RefCell::new(AmpState::default()));
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].source.get(), Source::FollowZone1);
        let rc5_data= get_rc5command_data(RC5Command::BD);
        let data = vec![rc5_data.0, rc5_data.1];
        assert_eq!(
            create_command_response(&Request::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, data.clone()).unwrap(), amp_state_ptr.clone(), None).unwrap(),
            Response::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, data).unwrap());
        assert_eq!(amp_state_ptr.borrow().zones[&ZoneNumber::Two].source.get(), Source::BD);
    }

}
