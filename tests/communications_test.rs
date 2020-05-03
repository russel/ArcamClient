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

mod start_avr850;

use std::cell::RefCell;
use std::rc::Rc;

use gio;
use gio::prelude::*;
use gtk;
use gtk::prelude::*;

use arcamclient::arcam_protocol::{
    ZoneNumber, Command, AnswerCode,
    REQUEST_VALUE,
    create_request, parse_response
};
use arcamclient::comms_manager;
use arcamclient::control_window::ControlWindow;
use arcamclient::functionality;

use start_avr850::PORT_NUMBER;

// Replacement for functionality::check_status_and_send_request for testing.
// NB The definition is changed during testing to support UI testing, so we have
// to provide a definition more like the non-test version – but without any UI activity.
fn check_status_and_send_request(control_window: &Rc<ControlWindow>, request: &[u8]) {
    if control_window.connect.get_active() {
        control_window.to_comms_manager.borrow_mut().as_ref().unwrap().send(request.to_vec());
    } else {
        eprintln!("There is no socket connection to send on, sending: {:?}", request);
    }
}

async fn terminate_application(control_window: Rc<ControlWindow>) {
    control_window.window.get_application().unwrap().quit();
}

fn with_dummy_control_window_connected_to_mock_avr850(code: &'static dyn Fn(Rc<ControlWindow>)) {
    let application = gtk::Application::new(Some("uk.org.winder.arcamclient"), gio::ApplicationFlags::empty()).unwrap();
    application.connect_startup(
        move |app| {
            let control_window = Rc::new(ControlWindow {
                window: gtk::ApplicationWindow::new(app),
                address: Default::default(),
                connect: Default::default(),
                brightness: gtk::Label::new(Some("dummy")),
                zone_1_adjustment: gtk::Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 10.0),
                zone_1_mute: Default::default(),
                zone_2_adjustment: gtk::Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 10.0),
                zone_2_mute: Default::default(),
                to_comms_manager: RefCell::new(None)
            });
            eprintln!("~~~~  with_dummy_control_window: making connection to {}", unsafe { PORT_NUMBER });
            let (tx_from_comms_manager, rx_from_comms_manager) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);
            rx_from_comms_manager.attach(None, {
                let c_w = control_window.clone();
                move |datum| {
                    functionality::process_response(&c_w, datum);
                    Continue(true)
                }
            });
            match comms_manager::connect_to_amp( &tx_from_comms_manager, "127.0.0.1", unsafe { PORT_NUMBER }) {
                Ok(s) => {
                    *control_window.to_comms_manager.borrow_mut() = Some(s);
                },
                Err(e) => panic!("~~~~ with_dummy_control_window: failed to connect to the mock amp."),
            }
            glib::source::timeout_add_seconds_local(1, {
                let c_w = control_window.clone();
                let mut count = 0;
                move || {
                    count += 1;
                    eprintln!("~~~~  with_dummy_control_window: count has the value {}", count);
                    if c_w.connect.get_active() {
                        Continue(count <= 10)
                    } else {
                        eprintln!("~~~~  with_dummy_control_window: running the supplied code.");
                        code(c_w.clone());
                        eprintln!("~~~~  with_dummy_control_window: send termination signal.");
                        glib::MainContext::default().spawn_local(terminate_application(c_w.clone()));
                        Continue(false)
                    }
                }
            });
        }
    );
    application.connect_activate(|_|{}); // Avoids a warning.
    eprintln!("~~~~  with_dummy_control_window: starting the application event loop.");
    application.run(&[]);
    eprintln!("~~~~  with_dummy_control_window: the application event loop has terminated.");
}

#[test]
fn connect_to_mock_avr850() {
    eprintln!("~~~~  connect_to_mock_avr850: starting connection to port {}", unsafe { PORT_NUMBER });
    with_dummy_control_window_connected_to_mock_avr850(
        &|c_w| {
            assert!(c_w.to_comms_manager.borrow().is_some());
        });
}

/*
#[test]
fn send_brightness_request() {
    eprintln!("~~~~  send_brightness_request: starting connection to port {}", unsafe { PORT_NUMBER });
    with_dummy_control_window_connected_to_mock_avr850(
        &|c_w| {
            assert!(c_w.to_comms_manager.borrow().is_some());
            check_status_and_send_request(&c_w, &create_request(ZoneNumber::One, Command::DisplayBrightness, &[REQUEST_VALUE]).unwrap());
        });
}
 */
