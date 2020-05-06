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

use arcamclient::arcam_protocol::{AnswerCode, Command, ZoneNumber, create_request, Brightness};
use arcamclient::comms_manager;
use arcamclient::control_window;
use arcamclient::functionality;

async fn terminate_application(control_window: Rc<control_window::ControlWindow>) {
    control_window.get_application().unwrap().quit();
}

use start_avr850::PORT_NUMBER;

#[test]
fn system_test_with_mock_amp() {
    let application = gtk::Application::new(Some("uk.org.winder.arcamclient.system_test"), gio::ApplicationFlags::empty()).unwrap();
    application.connect_startup(move |app| {
        let control_window = control_window::ControlWindow::new(&app);

        control_window.set_address("127.0.0.1");
        control_window.get_connect_chooser().set_active(true);

        glib::source::timeout_add_seconds_local(1, {
            let c_w = control_window.clone();
            move ||{

                //assert!(c_w.get_connect_display_value());
                assert_eq!(c_w.get_brightness_display_value(), Brightness::Level1);
                assert_eq!(c_w.get_volume_display_value(ZoneNumber::One), 30);
                assert!(c_w.get_mute_display_value(ZoneNumber::One));
                assert_eq!(c_w.get_volume_display_value(ZoneNumber::Two), 20);
                assert!(!c_w.get_mute_display_value(ZoneNumber::Two));

                glib::source::timeout_add_seconds_local(1, {
                    let cw = c_w.clone();
                    move || {
                        cw.get_application().unwrap().quit();
                        Continue(false)
                    }
                });

                Continue(false)
            }
        });
    });
    application.connect_activate(|_|{}); // Avoids a warning.
    eprintln!("ui_test::ui_test: starting the application event loop.");
    application.run(&[]);
    eprintln!("ui_test::ui_test: the application event loop has terminated.");
}
