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

// This module is a Mediator/Façade (roughly, not as per Gang of Four book in which
// patterns are about classes) between the UI code (control_window module) and the
// communications code (comms_manager module).

use std::rc::Rc;

use gtk;
use gtk::prelude::*;

use futures::channel::mpsc::Sender;

use num_derive::FromPrimitive;  // Apparently unused, but it is necessary.
use num_traits::FromPrimitive;

use crate::arcam_protocol::{AnswerCode, Command, ZoneNumber, REQUEST_VALUE, create_request, parse_response};
use crate::comms_manager;
use crate::control_window::{ControlWindow, ConnectedState};

pub type RequestTuple = (ZoneNumber, Command, Vec<u8>);
pub type ResponseTuple = (ZoneNumber, Command, AnswerCode, Vec<u8>);

/// Connect to an Arcam amp at the address given.
pub fn connect_to_amp(
    to_control_window: &glib::Sender<Vec<u8>>,
    address: &str,
    port_number: u16
) -> Result<futures::channel::mpsc::Sender<Vec<u8>>, String> {
    eprintln!("functionality::connect_to_amp: connecting to {}:{}", address, port_number);
    let x = comms_manager::connect_to_amp(to_control_window, address, port_number);
    match &x {
        Ok(y) => eprintln!("functionality::connect_to_amp: got Ok result {:p}", y),
        Err(e) => eprintln!("functionality::connect_to_amp: got Err result – {:?}", e),
    }
    x
}

/// Terminate the current connection.
pub fn disconnect_from_amp() {
    // TODO What to do to disconnect from the amp?
}

pub fn send_request(sender: &mut Sender<Vec<u8>>, request: &[u8]) {
    eprintln!("functionality::send_request: send message to amp {:?}", request);
    match sender.try_send(request.to_vec()) {
        Ok(_) => {},
        Err(e) => eprintln!("functionality::send_request: failed to send packet – {:?}", e),
    }
}

pub fn get_source_from_amp(sender: &mut Sender<Vec<u8>>) {
    send_request(sender, &create_request(ZoneNumber::One, Command::RequestCurrentSource, &[REQUEST_VALUE]).unwrap());
}


pub fn get_brightness_from_amp(sender: &mut Sender<Vec<u8>>) {
    send_request(sender, &create_request(ZoneNumber::One, Command::DisplayBrightness, &[REQUEST_VALUE]).unwrap());
}

pub fn get_mute_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    send_request(sender, &create_request(zone, Command::RequestMuteStatus, &[REQUEST_VALUE]).unwrap());
}

pub fn set_mute_on_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber, off: bool) {
    eprintln!("set zone 1 mute state to {}", off);
}

pub fn get_volume_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    send_request(sender, &create_request(zone, Command::SetRequestVolume, &[REQUEST_VALUE]).unwrap());
}

pub fn set_volume_on_amp(sender: &mut Sender<Vec<u8>>, zone:ZoneNumber, value: f64) {
    let volume = value as u8;
    assert!(volume < 100);
    send_request(sender, &create_request(zone, Command::SetRequestVolume, &[volume]).unwrap());
}

pub fn initialise_control_window(sender: &mut Sender<Vec<u8>>) {
    get_source_from_amp(sender);
    get_brightness_from_amp(sender);
    get_volume_from_amp(sender, ZoneNumber::One);
    get_mute_from_amp(sender, ZoneNumber::One);
    get_volume_from_amp(sender, ZoneNumber::Two);
    get_mute_from_amp(sender, ZoneNumber::Two);
}

fn handle_response(control_window: &Rc<ControlWindow>, zone: ZoneNumber, cc: Command, ac: AnswerCode, datum: &Vec<u8>) {
    eprintln!("functionality::handle_response: dealing with response ({:?}, {:?}, {:?}, {:?})", zone, cc, ac, &datum);
    // TODO Deal with non-StatusUpdate packets.
    assert_eq!(ac, AnswerCode::StatusUpdate);
    match cc {
        Command::DisplayBrightness => {
            assert_eq!(datum.len(), 1);
            control_window.set_brightness_display(FromPrimitive::from_u8(datum[0]).unwrap())
        },
        Command::SetRequestVolume => {
            assert_eq!(datum.len(), 1);
            control_window.set_volume_display(zone, datum[0] as f64);
        },
        Command::RequestMuteStatus => {
            assert_eq!(datum.len(), 1);
            control_window.set_mute_display(zone, FromPrimitive::from_u8(datum[0]).unwrap());
        },
        Command::RequestDABStation => {
            assert_eq!(datum.len(), 16);
            let message = match String::from_utf8(datum.to_vec()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { eprintln!("functionality::handle_response: failed to process {:?} – {:?}", &datum, e); "".to_string()},
            };
            eprintln!("functionality::handle_response: got the station name: {}", message);
            control_window.set_radio_station_display(&message);
        }
        Command::ProgrammeTypeCategory => {
            assert_eq!(datum.len(), 16);
            let message = match String::from_utf8(datum.to_vec()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { eprintln!("functionality::handle_response: failed to process {:?} – {:?}", &datum, e); "".to_string()},
            };
            eprintln!("functionality::handle_response: got the station type: {}", message);
            control_window.set_music_type_display(&message);
        }
        Command::RequestRDSDLSInformation => {
            assert_eq!(datum.len(), 129);
            let index_of_nul = match datum.iter().position(|x| *x == 0u8) {
                Some(i) => i,
                None => { eprintln!("functionality::handle_response: failed to find a nul character in the array."); 129 },
            };
            let message = match String::from_utf8(datum[1..index_of_nul].to_vec()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { eprintln!("functionality::handle_response: failed to get a string – {}", e); "".to_string() },
            };
            eprintln!("functionality::handle_response: got the RDS DLS: {}", message);
            control_window.set_rds_dls(&message);
        }
        Command::RequestCurrentSource => {
            assert_eq!(datum.len(), 1);
            control_window.set_source_display(zone, FromPrimitive::from_u8(datum[0]).unwrap());
        },
        x => eprintln!("functionality::handle_response: failed to deal with command {:?}", x),
    };
    control_window.set_connect_display(ConnectedState::Connected);
}

pub fn try_parse_of_response_data(control_window: &Rc<ControlWindow>, queue: &mut Vec<u8>) -> bool {
    eprintln!("functionality::try_parse_of_response_data: starting parse on queue: {:?}", &queue);
    match parse_response(&queue) {
        Ok((zone, cc, ac, data, count)) => {
            eprintln!("functionality::try_parse_of_response_data: got a successful parse of a packet. {:?}", (zone, cc, ac, &data));
            for _ in 0..count { queue.remove(0); }
            eprintln!("functionality::try_parse_of_response_data: updated buffer {:?}", queue);
            handle_response(control_window, zone, cc, ac, &data);
            true
        },
        Err(e) => {
            eprintln!("functionality::try_parse_of_response_data: failed to parse a packet from {:?}.", queue);
            match e {
                "Insufficient bytes to form a packet." => {},
                _ => panic!("XXXXX {}", e),
            };
            false
        },
    }
}
