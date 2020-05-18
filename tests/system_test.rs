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

// Need to start a mock AVR850.
mod start_avr850;

use std::rc::Rc;

use gio;
use gio::prelude::*;
use gtk;
use gtk::prelude::*;

use futures;
use futures::StreamExt;

use arcamclient::arcam_protocol::{AnswerCode, Brightness, Command, MuteState, PowerState, ZoneNumber};
use arcamclient::comms_manager;
use arcamclient::control_window::{ConnectedState, ControlWindow};
use arcamclient::functionality;

use start_avr850::PORT_NUMBER;

#[test]
fn system_test_with_mock_amp() {
    let application = gtk::Application::new(Some("uk.org.winder.arcamclient.system_test"), gio::ApplicationFlags::empty()).unwrap();
    application.connect_startup(move |app| {
        let control_window = ControlWindow::new(&app, Some(unsafe { PORT_NUMBER }));

        control_window.set_address("127.0.0.1");
        control_window.get_connect_chooser().set_active(true);

        // Have to wait for long enough for all the activity of initialising to settle.
        glib::timeout_add_local(650, {
            let a = app.clone();
            let c_w = control_window.clone();
            let mut first_run = true;

            move ||{

                if first_run {
                    first_run = false;
                    Continue(true)
                } else {

                    assert_eq!(c_w.get_connect_display_value(), ConnectedState::Connected);
                    assert_eq!(c_w.get_brightness_display_value(), Brightness::Level2);
                    assert_eq!(c_w.get_power_display_value(ZoneNumber::One), PowerState::On);
                    assert_eq!(c_w.get_volume_display_value(ZoneNumber::One), 30);
                    assert_eq!(c_w.get_mute_display_value(ZoneNumber::One), MuteState::NotMuted);
                    assert_eq!(c_w.get_power_display_value(ZoneNumber::Two), PowerState::Standby);
                    assert_eq!(c_w.get_volume_display_value(ZoneNumber::Two), 20);
                    assert_eq!(c_w.get_mute_display_value(ZoneNumber::Two), MuteState::NotMuted);

                    glib::idle_add_local({
                        let aa = a.clone();
                        move || {
                            aa.quit();
                            Continue(false)
                        }
                    });
                    Continue(false)
                }
            }
        });
    });
    application.connect_activate(|_|{}); // Avoids a warning.
    application.run(&[]);
}
