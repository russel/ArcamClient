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

use std::cell::RefCell;
use std::rc::Rc;

use gio;
use gio::prelude::*;
use gtk;
use gtk::prelude::*;

use futures;
use futures::channel::mpsc::{Sender, Receiver};
use futures::StreamExt;
//use futures::TryStreamExt;
//use futures::AsyncRead;
//use futures_util::io::AsyncReadExt;

use arcamclient::arcam_protocol::{
    AnswerCode,Command, Source, ZoneNumber,
    REQUEST_VALUE,
    create_request, create_response, parse_response
};
use arcamclient::comms_manager;
use arcamclient::control_window::ControlWindow;
use arcamclient::functionality::{ResponseTuple, send_request};

use start_avr850::PORT_NUMBER;

#[test]
fn communications_test() {
    //  Start up an application, no need for a UI.
    let application = gtk::Application::new(Some("uk.org.winder.arcamclient.communications_test"), gio::ApplicationFlags::empty()).unwrap();
    application.connect_startup(move |app| {
        // Set up the mock AVR850 process.
        eprintln!("communications_test::communications_test: making connection to {}", unsafe { PORT_NUMBER });
        let (mut tx_queue, mut rx_queue) = futures::channel::mpsc::channel(10);
        let (tx_from_comms_manager, rx_from_comms_manager) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);
        rx_from_comms_manager.attach(None, move |datum| {
            eprintln!("communications_test::communications_test: got a response {:?}.", datum);
            match tx_queue.try_send(datum) {
                Ok(_) => {},
                Err(e) => assert!(false, e),
            };
            Continue(true)
        });
        let mut sender = match comms_manager::connect_to_amp( &tx_from_comms_manager, "127.0.0.1", unsafe { PORT_NUMBER }) {
            Ok(s) => {
                eprintln!("communications_test::communications_test: connected to 127.0.0.1:{:?}.", unsafe{ PORT_NUMBER });
                s
            },
            Err(e) => panic!("~~~~ communications_test: failed to connect to the mock amp."),
        };
        // Run the tests.
        glib::MainContext::default().spawn_local({
            let a = app.clone();
            async move {
                eprintln!("communications_test::communications_test: running the test code.");

                send_request(
                    &mut sender,
                    &create_request(ZoneNumber::One, Command::DisplayBrightness, &[REQUEST_VALUE]).unwrap());
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, create_response(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, &[0x01]).unwrap()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };

                send_request(
                    &mut sender,
                    &create_request(ZoneNumber::One, Command::SetRequestVolume, &[0x14]).unwrap()
                );
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, create_response(ZoneNumber::One, Command::SetRequestVolume, AnswerCode::StatusUpdate, &[0x14]).unwrap()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };

                send_request(
                    &mut sender,
                    &create_request(ZoneNumber::One, Command::RequestCurrentSource, &[REQUEST_VALUE]).unwrap()
                );
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, create_response(ZoneNumber::One, Command::RequestCurrentSource, AnswerCode::StatusUpdate, &[Source::TUNER as u8]).unwrap()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };

                // Terminate the application once all tests are run.
                eprintln!("communications_test::communications_test: send termination signal.");
                glib::source::timeout_add_seconds_local(1, {
                    let aa = a.clone();
                    move ||{
                        aa.quit();
                        Continue(false)
                    }
                });
            }
        });
    });
    application.connect_activate(|_|{}); // Avoids a warning.
    eprintln!("communications_test::communications_test: starting the application event loop.");
    application.run(&[]);
    eprintln!("communications_test::communications_test: the application event loop has terminated.");
}
