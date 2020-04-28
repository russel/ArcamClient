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

use crate::arcam_protocol::{Command, ZoneNumber, create_request};
use crate::comms_manager::send_to_amp;

pub fn get_brightness_from_amp() {
    send_to_amp(&create_request(ZoneNumber::One, Command::DisplayBrightness, &[0xf0]).unwrap());
}

pub fn get_zone_1_mute_from_amp() {
    send_to_amp(&create_request(ZoneNumber::One, Command::RequestMuteStatus, &[0xf0]).unwrap());
}

pub fn get_zone_1_volume_from_amp() {
    send_to_amp(&create_request(ZoneNumber::One, Command::SetRequestVolume, &[0xf0]).unwrap());
}

pub fn set_zone_1_volume_on_amp(value: f64) {
    let volume = value as u8;
    assert!(volume < 100);
    send_to_amp(&create_request(ZoneNumber::One, Command::SetRequestVolume, &[volume]).unwrap());
}

pub fn get_zone_2_mute_from_amp() {
    send_to_amp(&create_request(ZoneNumber::Two, Command::RequestMuteStatus, &[0xf0]).unwrap());
}

pub fn get_zone_2_volume_from_amp() {
    send_to_amp(&create_request(ZoneNumber::Two, Command::SetRequestVolume, &[0xf0]).unwrap());
}

pub fn set_zone_2_volume_on_amp(value: f64) {
    let volume = value as u8;
    assert!(volume < 100);
    send_to_amp(&create_request(ZoneNumber::Two, Command::SetRequestVolume, &[volume]).unwrap());
}

pub fn initialise_control_window() {
    get_brightness_from_amp();
    get_zone_1_volume_from_amp();
    get_zone_1_mute_from_amp();
    get_zone_2_volume_from_amp();
    get_zone_2_mute_from_amp();
}
