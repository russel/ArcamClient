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

use futures;
use futures::channel::mpsc::{Sender, Receiver};
use futures::TryStreamExt;

use arcamclient::arcam_protocol::{
    ZoneNumber, Command, AnswerCode,
    REQUEST_VALUE,
    create_request, parse_response
};
use arcamclient::comms_manager;
use arcamclient::control_window::ControlWindow;
use arcamclient::functionality::ResponseTuple;

use start_avr850::PORT_NUMBER;

// Replacement for functionality::check_status_and_send_request for testing.
// NB The definition is changed in functionality during testing to support UI testing,
// so we have to provide a definition more like the non-test version – but without any
// UI activity.
fn check_status_and_send_request(control_window: &Rc<ControlWindow>, request: &[u8]) {
    let (rx, tx) = futures::channel::mpsc::channel(10);
    let mut to_comms_manager = control_window.to_comms_manager.borrow_mut().replace(rx).unwrap();
    match to_comms_manager.try_send(request.to_vec()) {
        Ok(_) => eprintln!("~~~~  check_status_and_send_request: sent packet – {:?}", request),
        Err(e) => eprintln!("~~~~  check_status_and_send_request: failed to send packet – {:?}", e),
    }
    control_window.to_comms_manager.borrow_mut().replace(to_comms_manager);
}

async fn terminate_application(control_window: Rc<ControlWindow>) {
    control_window.window.get_application().unwrap().quit();
}

#[test]
fn communications_test() {
    //  Start up an application but using a dummy UI.
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
            // Set up the mock AVR850 process.
            eprintln!("~~~~  communications_test: making connection to {}", unsafe { PORT_NUMBER });
            let (tx_from_comms_manager, rx_from_comms_manager) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);
            rx_from_comms_manager.attach(None,
                move |datum| {
                    eprintln!("~~~~  communications_test: got a response {:?}.", datum);
                    assert_ne!(datum, (ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![0x01]));
                    Continue(true)
                }
            );
            match comms_manager::connect_to_amp( &tx_from_comms_manager, "127.0.0.1", unsafe { PORT_NUMBER }) {
                Ok(s) => {
                    eprintln!("~~~~  communications_test: connected to 127.0.0.1:{:?}.", unsafe{ PORT_NUMBER });
                    *control_window.to_comms_manager.borrow_mut() = Some(s);
                },
                Err(e) => panic!("~~~~ communications_test: failed to connect to the mock amp."),
            }
            // Run the tests, but only after everything is working.
            glib::source::timeout_add_seconds_local(2, {
                let c_w = control_window.clone();
                move || {
                    if c_w.to_comms_manager.borrow().is_some() {
                        eprintln!("~~~~  communications_test: running the test code.");

                        check_status_and_send_request(&c_w, &create_request(ZoneNumber::One, Command::DisplayBrightness, &[REQUEST_VALUE]).unwrap());

                        // now ned to async block on getting a result to test.

                        eprintln!("~~~~  communications_test: send termination signal.");
                        glib::source::timeout_add_seconds_local(5, {
                            let cw = c_w.clone();
                            move ||{
                                cw.window.get_application().unwrap().quit();
                                Continue(false)
                            }
                        });

                        Continue(false)
                    } else {
                        Continue(true)
                    }
                }
            });
        }
    );
    application.connect_activate(|_|{}); // Avoids a warning.
    eprintln!("~~~~  communications_test: starting the application event loop.");
    application.run(&[]);
    eprintln!("~~~~  communications_test: the application event loop has terminated.");
}
