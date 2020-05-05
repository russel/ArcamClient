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
// communications code (comms_manager module). This allows for altered function
// definitions to support integration testing.

use std::rc::Rc;
#[cfg(test)]
use std::sync::Mutex;

//#[cfg(not(test))]
//use glib;
//use glib::prelude::*;
use gtk;
use gtk::prelude::*;

#[cfg(test)]
use lazy_static::lazy_static;

use crate::arcam_protocol::{AnswerCode, Command, ZoneNumber, REQUEST_VALUE, create_request};
use crate::comms_manager;
use crate::control_window::ControlWindow;

pub type RequestTuple = (ZoneNumber, Command, Vec<u8>);
pub type ResponseTuple = (ZoneNumber, Command, AnswerCode, Vec<u8>);

/// Connect to an Arcam amp at the address given.
pub fn connect_to_amp(
    to_control_window: &glib::Sender<ResponseTuple>,
    address: &str,
    port_number: u16
) -> Result<futures::channel::mpsc::Sender<Vec<u8>>, String> {
    comms_manager::connect_to_amp(to_control_window, address, port_number)
}

/// Terminate the current connection.
pub fn disconnect_from_amp() {
}

// For UI integration testing replace the function that sends a packet to the amplifier
// with a function that sends the packet to a queue that can be checked by the testing
// code.
//
// When compiling the ui_test crate we need these definitions. However when compiling
// the communications_test crate we need a different definition, more like the non-test
// application definition. Fortunately, we only need the updated definition for here
// for the ui_test, the definition needed for communication_test can be in that file/crate.

fn check_status_and_send_request(control_window: &Rc<ControlWindow>, request: &[u8]) {
    if control_window.connect.get_active() {
        eprintln!("functionality::check_status_and_send_request: send message to amp {:?}", request);
        //  TODO How come mutable borrow works here?
        //  TODO Why is the argument to replace here not an Option?
        // Cannot use the content of control_window.to_comms_manager as mutable so get
        // it out first. Need a dummy Sender because there seems to be implicit
        // unwrapping of the Option in the replace function. This is clearly wrong,
        // we should be able to replace with None.
        let (rx, tx) = futures::channel::mpsc::channel(10);
        let mut to_comms_manager = control_window.to_comms_manager.borrow_mut().replace(rx).unwrap();
        match to_comms_manager.try_send(request.to_vec()) {
            Ok(_) => {},
            Err(e) => eprintln!("functionality::check_status_and_send_request: failed to send packet – {:?}", e),
        }
        control_window.to_comms_manager.borrow_mut().replace(to_comms_manager);
    } else {
        let dialogue = gtk::MessageDialog::new(
            None::<&gtk::Window>,
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Info,
            gtk::ButtonsType::Ok,
            "Not connected to an amplifier."
        );
        dialogue.run();
        dialogue.destroy();
    }
}

pub fn get_brightness_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::One, Command::DisplayBrightness, &[REQUEST_VALUE]).unwrap());
}

pub fn get_zone_1_mute_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::One, Command::RequestMuteStatus, &[REQUEST_VALUE]).unwrap());
}

pub fn set_zone_1_mute_on_amp(control_window: &Rc<ControlWindow>, off: bool) {
    eprintln!("set zone 1 mute state to {}", off);
}

pub fn get_zone_1_volume_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::One, Command::SetRequestVolume, &[REQUEST_VALUE]).unwrap());
}

pub fn set_zone_1_volume_on_amp(control_window: &Rc<ControlWindow>, value: f64) {
    let volume = value as u8;
    assert!(volume < 100);
    check_status_and_send_request(control_window, &create_request(ZoneNumber::One, Command::SetRequestVolume, &[volume]).unwrap());
}

pub fn get_zone_2_mute_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::Two, Command::RequestMuteStatus, &[REQUEST_VALUE]).unwrap());
}

pub fn set_zone_2_mute_on_amp(control_window: &Rc<ControlWindow>, off: bool) {
    eprintln!("set zone 2 mute state to {}", off);
}

pub fn get_zone_2_volume_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::Two, Command::SetRequestVolume, &[REQUEST_VALUE]).unwrap());
}

pub fn set_zone_2_volume_on_amp(control_window: &Rc<ControlWindow>, value: f64) {
    let volume = value as u8;
    assert!(volume < 100);
    check_status_and_send_request(control_window, &create_request(ZoneNumber::Two, Command::SetRequestVolume, &[volume]).unwrap());
}

pub fn initialise_control_window(control_window: &Rc<ControlWindow>) {
    get_brightness_from_amp(control_window);
    get_zone_1_volume_from_amp(control_window);
    get_zone_1_mute_from_amp(control_window);
    get_zone_2_volume_from_amp(control_window);
    get_zone_2_mute_from_amp(control_window);
}

pub fn process_response(control_window: &Rc<ControlWindow>, datum: ResponseTuple) {
    let (zone, cc, ac, value) = datum;
    // TODO Deal with non-StatusUpdate packets.
    assert_eq!(ac, AnswerCode::StatusUpdate);
    match cc {
        Command::DisplayBrightness => {
            assert_eq!(value.len(), 1);
            control_window.set_brightness(value[0])
        },
        Command::SetRequestVolume => {
            assert_eq!(value.len(), 1);
            match zone {
                ZoneNumber::One => control_window.set_zone_1_volume(value[0]),
                ZoneNumber::Two => control_window.set_zone_2_volume(value[0]),
            }},
        Command::RequestMuteStatus => {
            assert_eq!(value.len(), 1);
            match zone {
                ZoneNumber::One => control_window.set_zone_1_mute(value[0]),
                ZoneNumber::Two => control_window.set_zone_2_mute(value[0]),
            }},
        Command::RequestDABStation => {
            assert_eq!(value.len(), 16);
            let message = match String::from_utf8(value.to_vec()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { eprintln!("functionality::process_response: failed to process {:?} – {:?}", value, e); "".to_string()},
            };
            eprintln!("functionality::process_response: got the station name: {}", message);
        }
        Command::ProgrammeTypeCategory => {
            assert_eq!(value.len(), 16);
            let message = match String::from_utf8(value.to_vec()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { eprintln!("functionality::process_response: failed to process {:?} – {:?}", value, e); "".to_string()},
            };
            eprintln!("functionality::process_response: got the station type: {}", message);
        }
        Command::RequestRDSDLSInformation => {
            assert_eq!(value.len(), 129);
            let index_of_nul = match value.iter().position(|x| *x == 0u8) {
                Some(i) => i,
                None => { eprintln!("Failed to find a nul character in the array."); 129 },
            };
            let message = match String::from_utf8(value[1..index_of_nul].to_vec()) {
                Ok(s) => s.trim().to_string(),
                Err(e) => { eprintln!("functionality::process_response: failed to get a string – {}", e); "".to_string() },
            };
            eprintln!("functionality::process_response: got the RDS DLS: {}", message);
        }
        _ => {},
    };
}
