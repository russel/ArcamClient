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

use arcamclient::arcam_protocol::{AnswerCode, Brightness, Command, Request, Response, Source, ZoneNumber, REQUEST_QUERY, RC5Command, get_rc5command_data};
use arcamclient::comms_manager;
use arcamclient::control_window::ControlWindow;
use arcamclient::functionality::{ResponseTuple, get_brightness_from_amp, get_source_from_amp, send_request_bytes, set_volume_on_amp, set_source_on_amp};

use start_avr850::PORT_NUMBER;

// GTK+ is not thread safe and starting an application requires access to the default
// context. This means we cannot run multiple Rust tests since they are multi-threaded.
// It is possible to run the tests single threaded, but that runs into problems. All in
// all it seems best to run all the tests within a single application. Messy as it means
// there is coupling between the tests – any changes made to the mock AVR850 state
// during a test is there for all subsequent tests.

#[test]
fn communications_test() {
    //  Start up an application, no need for a UI.
    let application = gtk::Application::new(Some("uk.org.winder.arcamclient.communications_test"), gio::ApplicationFlags::empty()).unwrap();
    application.connect_startup(move |app| {
        // Set up connection to the mock AVR850 process.
        let (mut tx_queue, mut rx_queue) = futures::channel::mpsc::channel(10);
        let (tx_from_comms_manager, rx_from_comms_manager) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);
        rx_from_comms_manager.attach(None, move |datum| {
            match tx_queue.try_send(datum) {
                Ok(_) => {},
                Err(e) => assert!(false, e),
            };
            Continue(true)
        });
        let mut sender = match comms_manager::connect_to_amp( &tx_from_comms_manager, "127.0.0.1", unsafe { PORT_NUMBER }) {
            Ok(s) => s,
            Err(e) => panic!("~~~~ communications_test: failed to connect to the mock amp."),
        };
        // Run the tests.
        //
        // Currently there is an assumption of synchronous request/response. A real
        // AVR 850 does not provide such a guarantee, the question is whether the
        // mock AVR850 does.
        glib::MainContext::default().spawn_local({
            let a = app.clone();
            async move {

                get_brightness_from_amp(&mut sender);
                // TODO It seems that the following .await causes the whole to terminate. Most times, but not always.
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Response::new(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![Brightness::Level2 as u8]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };

                set_volume_on_amp(&mut sender, ZoneNumber::One, 20);
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Response::new(ZoneNumber::One, Command::SetRequestVolume, AnswerCode::StatusUpdate, vec![0x14]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };

                get_source_from_amp(&mut sender, ZoneNumber::One);
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Response::new(ZoneNumber::One, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::TUNER as u8]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };

                // Send a multi-packet request. Do this by calling the comms_manage function directly.
                let mut buffer = Request::new(ZoneNumber::One, Command::DisplayBrightness, vec![REQUEST_QUERY]).unwrap().to_bytes();
                buffer.append(&mut Request::new(ZoneNumber::One, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap().to_bytes());
                send_request_bytes(&mut sender, &buffer);
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Response::new(ZoneNumber::One, Command::DisplayBrightness, AnswerCode::StatusUpdate, vec![0x01]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Response::new(ZoneNumber::One, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::TUNER as u8]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };

                // Set Zone 2 to CD and then to FollowZone1
                set_source_on_amp(&mut sender, ZoneNumber::Two, Source::CD);
                set_source_on_amp(&mut sender, ZoneNumber::Two, Source::FollowZone1);
                match rx_queue.next().await {
                    Some(s) => {
                        let rc5_command = get_rc5command_data(RC5Command::CD);
                        let rc5_data = vec![rc5_command.0, rc5_command.1];
                        assert_eq!(s, Response::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, rc5_data).unwrap().to_bytes())
                    },
                    None => assert!(false, "Failed to get a value from the response queue."),
                };
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Response::new(ZoneNumber::Two, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::CD as u8]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };
                match rx_queue.next().await {
                    Some(s) => {
                        let rc5_command = get_rc5command_data(RC5Command::SetZone2ToFollowZone1);
                        let rc5_data = vec![rc5_command.0, rc5_command.1];
                        assert_eq!(s, Response::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, AnswerCode::StatusUpdate, rc5_data).unwrap().to_bytes())
                    },
                    None => assert!(false, "Failed to get a value from the response queue."),
                };
                match rx_queue.next().await {
                    Some(s) => assert_ne!(s, Response::new(ZoneNumber::Two, Command::RequestCurrentSource, AnswerCode::StatusUpdate, vec![Source::FollowZone1 as u8]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };

                assert_eq!(1, 2);  // This line fails to execute.

                // Add the application quit event once there is no other event.
                glib::idle_add_local({
                    let aa = a.clone();
                    move ||{
                        if glib::MainContext::default().pending() {
                            Continue(true)
                        } else {
                            aa.quit();
                            Continue(false)
                        }
                    }
                });
            }
        });
    });
    application.connect_activate(|_|{}); // Avoids a warning.
    application.run(&[]);
}
