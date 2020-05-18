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
use std::thread::sleep;
use std::time::Duration;

use gtk;
use gtk::prelude::*;

use futures::channel::mpsc::Sender;

use num_derive::FromPrimitive;  // Apparently unused, but it is necessary.
use num_traits::FromPrimitive;

use crate::arcam_protocol::{AnswerCode, Command, MuteState, PowerState, RC5Command, Request, Response, Source, ZoneNumber,
                            REQUEST_VALUE, get_rc5command_data};
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

pub fn send_request_bytes(sender: &mut Sender<Vec<u8>>, request: &Vec<u8>) {
    eprintln!("functionality::send_request: send message to amp {:?}", request);
    match sender.try_send(request.to_vec()) {
        Ok(_) => {},
        Err(e) => eprintln!("functionality::send_request_bytes: failed to send packet – {:?}", e),
    }
}

pub fn send_request(sender: &mut Sender<Vec<u8>>, request: &Request) {
    eprintln!("functionality::send_request: send message to amp {:?}", request);
    send_request_bytes(sender, &request.to_bytes());
}

pub fn get_brightness_from_amp(sender: &mut Sender<Vec<u8>>) {
    let request = Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![REQUEST_VALUE]).unwrap();
    send_request(sender, &request);
}

pub fn get_power_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    let request = Request::new(zone, Command::Power, vec![REQUEST_VALUE]).unwrap();
    send_request(sender, &request);
}

pub fn set_power_on_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber, power: PowerState) {
    let rc5_data = get_rc5command_data(
        if zone == ZoneNumber::One {
            if power == PowerState::On { RC5Command::PowerOn } else { RC5Command::PowerOff }
        } else {
            if power == PowerState::On { RC5Command::Zone2PowerOn } else { RC5Command::Zone2PowerOff }
        }
    );
    let data = vec![rc5_data.0, rc5_data.1];
    let request = Request::new(zone, Command::SimulateRC5IRCommand, data).unwrap();
    send_request(sender, &request);
    get_power_from_amp(sender, zone);
}

pub fn get_source_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    let request = Request::new(zone, Command::RequestCurrentSource, vec![REQUEST_VALUE]).unwrap();
    send_request(sender, &request);
}

pub fn set_source_on_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber, source: Source) {
    let rc5_command = match source {
        Source::FollowZone1 => RC5Command::SetZone2ToFollowZone1,
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
    let data = vec![rc5_data.0, rc5_data.1];
    let request = Request::new(zone, Command::SimulateRC5IRCommand, data).unwrap();
    send_request(sender, &request);
    get_source_from_amp(sender, zone);
}


pub fn get_mute_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    let request = Request::new(zone, Command::RequestMuteStatus, vec![REQUEST_VALUE]).unwrap();
    send_request(sender, &request);
}

pub fn set_mute_on_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber, mute: MuteState) {
    let rc5_data = get_rc5command_data(
        if zone == ZoneNumber::One {
            if mute == MuteState::Muted { RC5Command::MuteOn } else { RC5Command::MuteOff }
        } else {
            if mute == MuteState::Muted { RC5Command::Zone2MuteOn } else { RC5Command::Zone2MuteOff }
        }
    );
    let data = vec![rc5_data.0, rc5_data.1];
    let request = Request::new(zone, Command::SimulateRC5IRCommand, data).unwrap();
    send_request(sender, &request);
    get_mute_from_amp(sender, zone);
}

pub fn get_volume_from_amp(sender: &mut Sender<Vec<u8>>, zone: ZoneNumber) {
    let request = Request::new(zone, Command::SetRequestVolume, vec![REQUEST_VALUE]).unwrap();
    send_request(sender, &request);
}

pub fn set_volume_on_amp(sender: &mut Sender<Vec<u8>>, zone:ZoneNumber, value: f64) {
    let volume = value as u8;
    assert!(volume < 100);
    let request = Request::new(zone, Command::SetRequestVolume, vec![volume]).unwrap();
    send_request(sender, &request);
}

pub fn initialise_control_window(sender: &mut Sender<Vec<u8>>) {
    // Experimental evidence indicates that a real AVR 850 cannot deal with six requests
    // being thrown at it quickly. It seems that it can cope with sending two at a time
    // with a short gap.
    glib::idle_add_local({
        let mut s = sender.clone();
        move || {
            get_brightness_from_amp(&mut s);
            get_power_from_amp(&mut s, ZoneNumber::One);
            get_power_from_amp(&mut s, ZoneNumber::Two);
            Continue(false)
        }
    });
    glib::timeout_add_local(250, {
        let mut s = sender.clone();
        let mut first_run = true;
        move || {
            if first_run {
                first_run = false;
                Continue(true)
            } else {
                get_source_from_amp(&mut s, ZoneNumber::One);
                get_volume_from_amp(&mut s, ZoneNumber::One);
                get_mute_from_amp(&mut s, ZoneNumber::One);
                Continue(false)
            }
        }
    });
    glib::timeout_add_local(500, {
        let mut s = sender.clone();
        let mut first_run = true;
        move || {
            if first_run {
                first_run = false;
                Continue(true)
            } else {
                get_source_from_amp(&mut s, ZoneNumber::Two);
                get_volume_from_amp(&mut s, ZoneNumber::Two);
                get_mute_from_amp(&mut s, ZoneNumber::Two);
                Continue(false)
            }
        }
    });
}

fn handle_response(control_window: &Rc<ControlWindow>, response: &Response) {
    eprintln!("functionality::handle_response: dealing with response {:?}", response);
    // TODO Deal with non-StatusUpdate packets.
    assert_eq!(response.ac, AnswerCode::StatusUpdate);
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
            control_window.set_volume_display(response.zone, response.data[0] as f64);
        },
        Command::RequestMuteStatus => {
            assert_eq!(response.data.len(), 1);
            control_window.set_mute_display(response.zone, FromPrimitive::from_u8(response.data[0]).unwrap());
        },
        Command::RequestDABStation => {
            assert_eq!(response.data.len(), 16);
            let message = match String::from_utf8(response.data.clone()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { eprintln!("functionality::handle_response: failed to process {:?} – {:?}", &response.data, e); "".to_string()},
            };
            eprintln!("functionality::handle_response: got the station name: {}", message);
            control_window.set_radio_station_display(&message);
        }
        Command::ProgrammeTypeCategory => {
            assert_eq!(response.data.len(), 16);
            let message = match String::from_utf8(response.data.clone()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { eprintln!("functionality::handle_response: failed to process {:?} – {:?}", &response.data, e); "".to_string()},
            };
            eprintln!("functionality::handle_response: got the station type: {}", message);
            control_window.set_music_type_display(&message);
        }
        Command::RequestRDSDLSInformation => {
            assert_eq!(response.data.len(), 129);
            let index_of_nul = match response.data.iter().position(|x| *x == 0u8) {
                Some(i) => i,
                None => { eprintln!("functionality::handle_response: failed to find a nul character in the array."); 129 },
            };
            let message = match String::from_utf8(response.data[1..index_of_nul].to_vec()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { eprintln!("functionality::handle_response: failed to get a string – {}", e); "".to_string() },
            };
            eprintln!("functionality::handle_response: got the RDS DLS: {}", message);
            control_window.set_rds_dls(&message);
        }
        Command::RequestCurrentSource => {
            assert_eq!(response.data.len(), 1);
            control_window.set_source_display(response.zone, FromPrimitive::from_u8(response.data[0]).unwrap());
        },
        x => eprintln!("functionality::handle_response: failed to deal with command {:?}", x),
    };
    control_window.set_connect_display(ConnectedState::Connected);
}

pub fn try_parse_of_response_data(control_window: &Rc<ControlWindow>, queue: &mut Vec<u8>) -> bool {
    eprintln!("functionality::try_parse_of_response_data: starting parse on queue: {:?}", &queue);
    match Response::parse_bytes(&queue) {
        Ok((response, count)) => {
            eprintln!("functionality::try_parse_of_response_data: got a successful parse of a packet. {:?}", response);
            for _ in 0..count { queue.remove(0); }
            eprintln!("functionality::try_parse_of_response_data: updated buffer {:?}", queue);
            handle_response(control_window, &response);
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
