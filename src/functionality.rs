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

//! This module provides various functions to be used from the UI code in the
//! [control_window](../control_window/index.html) module to send data (in the form of Arcam
//! protocol packets, see [arcam_protocol](../arcam_protocol/index.html) module) to the
//! [comms_manager](../comms_manager/index.html) module functions for forwarding to the
//! amplifier, and functions to be called by functions in the
//! [comms_manager](../comms_manager/index.html) module to transform bytes received from the
//! amplifier into Arcam protocol response packets and then to call functions in the
//! [control_window](../control_window/index.html) module to make changes to the UI.
//!
//! This module is, in effect, a Mediator/Façade module between the UI
//! ([control_window](../control_window/index.html) module) and the comms
//! ([comms_manager](../comms_manager/index.html) module). This is not Mediator or Façade in the
//! [Gang of Four](https://en.wikipedia.org/wiki/Design_Patterns) design patterns sense as that
//! is all about class structures in an object oriented system.

use std::rc::Rc;

use gtk;
use gtk::prelude::*;

use futures::channel::mpsc::Sender;

use log::debug;

#[allow(unused_imports)]  // Compiler misses the use in a derive.
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::arcam_protocol::{
    Command, MuteState, PowerState, RC5Command, Request, Response, Source, ZoneNumber,
    REQUEST_QUERY,
    get_rc5command_data
};
use crate::comms_manager;
use crate::control_window::{ControlWindow, ConnectedState};

//pub type RequestTuple = (ZoneNumber, Command, Vec<u8>);
//pub type ResponseTuple = (ZoneNumber, Command, AnswerCode, Vec<u8>);

/// Connect to an Arcam amp at the address given.
pub fn connect_to_amp(
    to_control_window: &glib::Sender<Vec<u8>>,
    address: &str,
    port_number: u16
) -> Result<futures::channel::mpsc::Sender<Vec<u8>>, String> {
    debug!("connect_to_amp:  Connecting to {}:{}.", address, port_number);
    let x = comms_manager::connect_to_amp(to_control_window, address, port_number);
    match &x {
        Ok(y) => debug!("connect_to_amp:  Got Ok result {:p}.", y),
        Err(e) => debug!("connect_to_amp:  Got Err result – {:?}.", e),
    }
    x
}

/// Terminate the current connection.
pub fn disconnect_from_amp() {
    // TODO What to do to disconnect from the amp?
}

/// Send a sequence of bytes to the comms manager (via the appropriate channel) for forwarding
/// to the amplifier.
pub fn send_request_bytes(sender: &mut Sender<Vec<u8>>, request: &Vec<u8>) {
    debug!("send_request_bytes:  Send message to amp {:?}.", request);
    match sender.try_send(request.to_vec()) {
        Ok(_) => {},
        Err(e) => debug!("send_request_bytes:  Failed to send packet – {:?}.", e),
    }
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to the comms manager (via the
/// appropriate channel) for forwarding to the amplifier.
pub fn send_request(sender: &mut Sender<Vec<u8>>, request: &Request) {
    debug!("send_request:  Send message to amp {:?}.", request);
    send_request_bytes(sender, &request.to_bytes());
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to respond with the state of the
/// brightness to the amplifier.
pub fn get_brightness_from_amp(sender: &mut Sender<Vec<u8>>) {
    send_request(sender, &Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![REQUEST_QUERY]).unwrap());
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to respond with the current power
/// state for the given zone to the amplifier.
pub fn get_power_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    send_request(sender, &Request::new(zone, Command::Power, vec![REQUEST_QUERY]).unwrap());
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to amend the power state of a given
/// zone to the amplifier.
pub fn set_power_on_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber, power: PowerState) {
    let rc5_data = get_rc5command_data(
        if zone == ZoneNumber::One {
            if power == PowerState::On { RC5Command::PowerOn } else { RC5Command::PowerOff }
        } else {
            if power == PowerState::On { RC5Command::Zone2PowerOn } else { RC5Command::Zone2PowerOff }
        }
    );
    send_request(sender, &Request::new(zone, Command::SimulateRC5IRCommand, vec![rc5_data.0, rc5_data.1]).unwrap());
    // SimulateRC5IRCommand commands do not respond with the changed status.
    get_power_from_amp(sender, zone);
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to respond with the volume for the
/// given zone to the amplifier.
pub fn get_volume_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    send_request(sender, &Request::new(zone, Command::SetRequestVolume, vec![REQUEST_QUERY]).unwrap());
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to amend the volume of a given zone
/// to the amplifier.
pub fn set_volume_on_amp(sender: &mut Sender<Vec<u8>>, zone:ZoneNumber, volume: u8) {
    assert!(volume < 100);
    send_request(sender, &Request::new(zone, Command::SetRequestVolume, vec![volume]).unwrap());
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to respond with the mute state for
/// the given zone to the amplifier.
pub fn get_mute_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    send_request(sender, &Request::new(zone, Command::RequestMuteStatus, vec![REQUEST_QUERY]).unwrap());
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to amend the mute state of a given
/// zone to the amplifier.
pub fn set_mute_on_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber, mute: MuteState) {
    let rc5_data = get_rc5command_data(
        if zone == ZoneNumber::One {
            if mute == MuteState::Muted { RC5Command::MuteOn } else { RC5Command::MuteOff }
        } else {
            if mute == MuteState::Muted { RC5Command::Zone2MuteOn } else { RC5Command::Zone2MuteOff }
        }
    );
    send_request(sender, &Request::new(zone, Command::SimulateRC5IRCommand, vec![rc5_data.0, rc5_data.1]).unwrap());
    // SimulateRC5IRCommand commands do not respond with the changed status.
    get_mute_from_amp(sender, zone);
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to respond with the source for the
/// given zone to the amplifier.
pub fn get_source_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    send_request(sender, &Request::new(zone, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap());
}

/// Send a [Request](../arcam_protocol/struct.Request.html) to amend the source of a given zone
/// to the amplifier.
pub fn set_source_on_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber, source: Source) {
    let rc5_command = match source {
        Source::FollowZone1 => {
            assert_eq!(zone, ZoneNumber::Two);
            RC5Command::SetZone2ToFollowZone1
        },
        Source::CD => RC5Command::CD,
        Source::BD => RC5Command::BD,
        Source::AV => RC5Command::AV,
        Source::SAT => RC5Command::Sat,
        Source::PVR => RC5Command::PVR,
        Source::VCR => RC5Command::VCR,
        Source::AUX => RC5Command::Aux,
        Source::DISPLAY => RC5Command::Display,
        Source::TUNER => RC5Command::Radio,
        Source::TUNERDAB => RC5Command::Radio,
        Source::NET => RC5Command::Net,
        Source::USB => RC5Command::USB,
        Source::STB => RC5Command::STB,
        Source::GAME => RC5Command::Game,
    };
    let rc5_data = get_rc5command_data(rc5_command);
    send_request(sender, &Request::new(zone, Command::SimulateRC5IRCommand, vec![rc5_data.0, rc5_data.1]).unwrap());
    // SimulateRC5IRCommand commands do not respond with the changed status.
    get_source_from_amp(sender, zone);
}

/// Send [Request](../arcam_protocol/struct.Request.html)s to the amplifier so as to get
/// [Response](../arcam_protocol/struct.Response.html)s from the amplifier so as to set all the
/// displays of the UI.
// Experimental evidence indicates that a real AVR 850 cannot deal with a large number
// of requests being sent to it at once. This means requests must be sent with a small
// time gap. The gap has been ascertained by rough experiment with an AVR850 rather
// than guesswork: 150 ms seems insufficient, 175 ms works sometimes, 200 ms seems
// mostly to work but not always, 225 ms seems to work always.
pub fn initialise_control_window(sender: &mut Sender<Vec<u8>>) {
    glib::timeout_add_local(225, {
        let mut s = sender.clone();
        let mut count = -1;
        move || {
            count += 1;
            match count {
                0 => { get_brightness_from_amp(&mut s); Continue(true) },
                1 => { get_power_from_amp(&mut s, ZoneNumber::One); Continue(true) },
                2=> { get_power_from_amp(&mut s, ZoneNumber::Two); Continue(true) },
                3 => { get_volume_from_amp(&mut s, ZoneNumber::One); Continue(true) },
                4 => { get_volume_from_amp(&mut s, ZoneNumber::Two); Continue(true) },
                5 => { get_mute_from_amp(&mut s, ZoneNumber::One); Continue(true) },
                6 => { get_mute_from_amp(&mut s, ZoneNumber::Two); Continue(true) },
                7 => { get_source_from_amp(&mut s, ZoneNumber::One); Continue(true) },
                8 => { get_source_from_amp(&mut s, ZoneNumber::Two); Continue(false) },
                _ => Continue(false),
            }
        }
    });
}

/// Deal with a [Response](../arcam_protocol/struct.Response.html) packet received from the
/// amplifier.
///
/// This function transforms [Response](../arcam_protocol/struct.Response.html)s into actions on
/// the UI.
fn handle_response(control_window: &Rc<ControlWindow>, response: &Response) {
    debug!("handle_response:  Dealing with response {:?}.", response);
    match response.cc {
        Command::Power => {
            assert_eq!(response.data.len(), 1);
            control_window.set_power_display(response.zone, FromPrimitive::from_u8(response.data[0]).unwrap());
        },
        Command::DisplayBrightness => {
            assert_eq!(response.data.len(), 1);
            control_window.set_brightness_display(FromPrimitive::from_u8(response.data[0]).unwrap())
        },
        Command::SetRequestVolume => {
            assert_eq!(response.data.len(), 1);
            control_window.set_volume_display(response.zone, response.data[0]);
        },
        Command::RequestMuteStatus => {
            assert_eq!(response.data.len(), 1);
            control_window.set_mute_display(response.zone, FromPrimitive::from_u8(response.data[0]).unwrap());
        },
        Command::RequestDABStation => {
            assert_eq!(response.data.len(), 16);
            let message = match String::from_utf8(response.data.clone()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { debug!("Failed to process {:?} – {:?}", &response.data, e); "".to_string() },
            };
            debug!("Got the station name: {}", message);
            control_window.set_radio_station_display(response.zone, &message);
        }
        Command::ProgrammeTypeCategory => {
            assert_eq!(response.data.len(), 16);
            let message = match String::from_utf8(response.data.clone()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { debug!("handle_response:  Failed to process {:?} – {:?}.", &response.data, e); "".to_string()},
            };
            debug!("handle_response:  Got the station type: {}.", message);
            control_window.set_music_type_display(response.zone, &message);
        }
        Command::DLSPDTInformation => {
            // An AVR850 appears to behave differently to the documentation. Documentation
            // says a 128 byte buffer with the string padded out with spaces. Reality indicates
            // a 129 byte buffer with a nul terminated string padded out with spaces. Implement
            // what the real AVR850 pumps out.
            assert_eq!(response.data.len(), 129);
            let index_of_nul = match response.data.iter().position(|x| *x == 0u8) {
                Some(i) => i,
                None => { debug!("handle_response:  Failed to find a nul character in the array."); 129 },
            };
            let message = match String::from_utf8(response.data[0..index_of_nul].to_vec()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { debug!("handle_response:  Failed to get a string – {}.", e); "".to_string() },
            };
            debug!("functionality::handle_response: got the DLS/PDT: {}.", message);
            control_window.set_dlspdt_information(response.zone, &message);
        }
        Command::RequestCurrentSource => {
            assert_eq!(response.data.len(), 1);
            control_window.set_source_display(response.zone, FromPrimitive::from_u8(response.data[0]).unwrap());
        },
        Command::SimulateRC5IRCommand => {
            // Responses to this Request Command provide no data on the state of the
            // amplifier, they just give the AnswerCode to the Request.
            assert_eq!(response.data.len(), 2);
            debug!("handle_response:  Got response for RC5 command {:?}.", RC5Command::from(&response.data));
        },
        x => debug!("handle_response:  Failed to deal with command {:?}.", x),
    };
    control_window.set_connect_display(ConnectedState::Connected);
}

/// Attempt to extract a [Response](../arcam_protocol/struct.Response.html) packet from the
/// queue of bytes received from the amplifier.
///
/// On a successful parse the bytes of the packet are removed from the queue and the (not
/// public) [handle_response](fn.handle_response.html) function is called to implement any
/// changes to the UI consequent on the data in the
/// [Response](../arcam_protocol/struct.Response.html).
pub fn try_parse_of_response_data(control_window: &Rc<ControlWindow>, queue: &mut Vec<u8>) -> bool {
    debug!("try_parse_of_response_data:  Starting parse on queue: {:?}.", &queue);
    match Response::parse_bytes(&queue) {
        Ok((response, count)) => {
            debug!("try_parse_of_response_data:  Got a successful parse of a packet – {:?}.", response);
            for _ in 0..count { queue.remove(0); }
            debug!("try_parse_of_response_data:  Updated buffer – {:?}.", queue);
            handle_response(control_window, &response);
            true
        },
        Err(e) => {
            debug!("try_parse_of_response_data:  Failed to parse a packet from {:?}.", queue);
            match e {
                "Insufficient bytes to form a packet." => {},
                _ => panic!("try_parse_of_response_data:  {}", e),
            };
            false
        },
    }
}
