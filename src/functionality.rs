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

use glib;
//use glib::prelude::*;
use gtk;
use gtk::prelude::*;

#[cfg(test)]
use lazy_static::lazy_static;

use crate::arcam_protocol::{AnswerCode, Command, ZoneNumber, create_request};
use crate::comms_manager::send_to_amp;
use crate::control_window::ControlWindow;

// For UI integration testing replace the function that sends a packet to the amplifier
// with a function that sends the packet to a queue that can be checked by the testing
// code.

#[cfg(not(test))]
fn check_status_and_send_request(control_window: &Rc<ControlWindow>, request: &[u8]) {
    if control_window.socket_connection.borrow().is_some() {
        eprintln!("Send message to amp {:?}", request);
        glib::MainContext::default().spawn_local(send_to_amp(control_window.clone(), request.to_vec()));
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

// When compiling the ui_test crate we need these definitions. However when compiling
// the communications_test crate we need a different definition, more like the above
// application definition. Is there any way of doing conditional compilation based on
// the module/crate name?

/*
#[cfg(test)]
lazy_static! {
pub static ref TO_COMMS_MANAGER: Mutex<Vec<Vec<u8>>> = Mutex::new(vec![]);
}

#[cfg(test)]
fn check_status_and_send_request(control_window: &Rc<ControlWindow>, request: &[u8]) {
    if control_window.socket_connection.borrow().is_some() {
        TO_COMMS_MANAGER.lock().unwrap().push(request.to_vec());
    }
}
*/

// When compiling the communications_test integration test module we need this
// definition, so that we send the message but do not do any UI activity on failure.

#[cfg(test)]
fn check_status_and_send_request(control_window: &Rc<ControlWindow>, request: &[u8]) {
    if control_window.socket_connection.borrow().is_some() {
        glib::MainContext::default().spawn_local(send_to_amp(control_window.clone(), request.to_vec()));
    } else {
        eprintln!("There is no socket connection to send on, sending: {:?}", request);
    }
}

pub fn get_brightness_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::One, Command::DisplayBrightness, &[0xf0]).unwrap());
}

pub fn get_zone_1_mute_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::One, Command::RequestMuteStatus, &[0xf0]).unwrap());
}

pub fn set_zone_1_mute_on_amp(control_window: &Rc<ControlWindow>, off: bool) {
    eprintln!("set zone 1 mute state to {}", off);
}

pub fn get_zone_1_volume_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::One, Command::SetRequestVolume, &[0xf0]).unwrap());
}

pub fn set_zone_1_volume_on_amp(control_window: &Rc<ControlWindow>, value: f64) {
    let volume = value as u8;
    assert!(volume < 100);
    check_status_and_send_request(control_window, &create_request(ZoneNumber::One, Command::SetRequestVolume, &[volume]).unwrap());
}

pub fn get_zone_2_mute_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::Two, Command::RequestMuteStatus, &[0xf0]).unwrap());
}

pub fn set_zone_2_mute_on_amp(control_window: &Rc<ControlWindow>, off: bool) {
    eprintln!("set zone 2 mute state to {}", off);
}

pub fn get_zone_2_volume_from_amp(control_window: &Rc<ControlWindow>) {
    check_status_and_send_request(control_window, &create_request(ZoneNumber::Two, Command::SetRequestVolume, &[0xf0]).unwrap());
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

// For communications integration testing replace the processing function that normally
// dispatches UI events with a function that puts the messages on a queue so that they
// can be checked by the testing code.

#[cfg(not(test))]
pub fn process_response(control_window: &Rc<ControlWindow>, zone: ZoneNumber, cc: Command, ac: AnswerCode, value: &[u8]) {
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
        _ => {},
    };
}

#[cfg(test)]
lazy_static! {
pub static ref FROM_COMMS_MANAGER: Mutex<Vec<(ZoneNumber, Command, AnswerCode, Vec<u8>)>> = Mutex::new(vec![]);
}

#[cfg(test)]
pub fn process_response(control_window: &Rc<ControlWindow>, zone: ZoneNumber, cc: Command, ac: AnswerCode, value: &[u8]) {
    FROM_COMMS_MANAGER.lock().unwrap().push((zone, cc, ac, value.to_vec()));
}
