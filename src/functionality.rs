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

use std::rc::Rc;

use gtk;
use gtk::prelude::*;

use crate::arcam_protocol::{AnswerCode, Command, ZoneNumber, create_request};
use crate::comms_manager::{SocketClient, send_to_amp};
use crate::control_window::ControlWindow;

fn check_status_and_send_request(control_window: &Rc<ControlWindow>, request: &[u8]) {
    match &*control_window.socket_client.borrow() {
        Some(s_c) => {
            send_to_amp(&s_c, request);
        },
        None => {
            let dialogue = gtk::MessageDialog::new(
                None::<&gtk::Window>,
                gtk::DialogFlags::MODAL,
                gtk::MessageType::Info,
                gtk::ButtonsType::Ok,
                "Not connected to an amplifier."
            );
            dialogue.run();
            dialogue.destroy();
        },
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

pub fn process_response(zone: ZoneNumber, cc: Command, ac: AnswerCode, value: &[u8]) {
    assert_eq!(ac, AnswerCode::StatusUpdate);
    match cc {
        Command::DisplayBrightness => {},
        Command::SetRequestVolume => {},
        _ => {},
    };
}
