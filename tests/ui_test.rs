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

// Do not need to start a mock AVR850 for this test.

use std::rc::Rc;

use gio;
use gio::prelude::*;
use gtk;
use gtk::prelude::*;

use futures;
use futures::StreamExt;

use arcamclient::arcam_protocol::{
    AnswerCode, Command, Source, RC5Command, Request, ZoneNumber,
    REQUEST_QUERY,
    get_rc5command_data,
};
use arcamclient::comms_manager;
use arcamclient::control_window;
use arcamclient::functionality;

// GTK+ is not thread safe and starting an application requires access to the default
// context. This means we cannot run multiple Rust tests since they are multi-threaded.
// It is possible to run the tests single threaded, but that runs into problems. All in
// all it seems best to run all the tests within a single application. Messy as it means
// there is coupling between the tests – any changes made to the mock AVR850 state
// during a test is there for all subsequent tests.

#[test]
fn ui_test() {
    let application = gtk::Application::new(Some("uk.org.winder.arcamclient.ui_test"), gio::ApplicationFlags::empty()).unwrap();
    application.connect_startup(move |app| {
        let control_window = control_window::ControlWindow::new(&app, None);
        // Make it seem there is a connection: connect to somewhere guaranteed to fail.
        // Use 127.0.0.2 because it is not 127.0.0.1 and yet is a loopback address.
        // This ensures the UI state initialisation required with no attempt to use an
        // mock AVR850.
        control_window.set_address("127.0.0.2");
        control_window.set_connect_chooser(true); // Won't set the display state, so…
        control_window.set_connect_display(control_window::ConnectedState::Connected); // …set it manually.
        // Replace the channel to the comms manager with one that we can use for checking
        // the data sent. This cuts off the comms manager so that it's state no longer
        // matters for the tests.
        let (tx_queue, mut rx_queue) = futures::channel::mpsc::channel::<Vec<u8>>(10);
        control_window.get_to_comms_manager_field().borrow_mut().replace(tx_queue);
        // Set some tests going.
        glib::MainContext::default().spawn_local({
            let a = app.clone();
            let c_w = control_window.clone();
            async move {

                println!("XXXX  starting the tests.");

                // This should trigger a change to a volume ScrollButton and therefore
                // send a message to the amp.
                c_w.set_volume_chooser(ZoneNumber::One, 20.0);
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![0x14]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the request queue."),
                };

                println!("XXXX  set_volume_chooser.");

                // Set Zone 2 to CD and then to FollowZone1
                c_w.set_source_chooser(ZoneNumber::Two, Source::CD);
                let rc5_command = get_rc5command_data(RC5Command::CD);
                let rc5_data = vec![rc5_command.0, rc5_command.1];
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Request::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, rc5_data).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the request queue."),
                };

                println!("XXXX  set_source_chooser CD responded.");

                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Request::new(ZoneNumber::Two, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the request queue."),
                };

                println!("XXXX  set_source_chooser CD queried.");

                c_w.set_source_chooser(ZoneNumber::Two, Source::FollowZone1);
                let rc5_command = get_rc5command_data(RC5Command::SetZone2ToFollowZone1);
                let rc5_data = vec![rc5_command.0, rc5_command.1];
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Request::new(ZoneNumber::Two, Command::SimulateRC5IRCommand, rc5_data).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the request queue."),
                };

                println!("XXXX  set_source_chooser FollowZone1 responded.");

                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Request::new(ZoneNumber::Two, Command::RequestCurrentSource, vec![REQUEST_QUERY]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the request queue."),
                };

                println!("XXXX  set_source_chooser FollowZone1 queried.");

                // Add the application quit event once there is no other event.
                //
                // Whilst this works locally and on GitLab, it fails on Travis-CI.
                // It appears that on Travis-CI glib::MainContext::default().pending()
                // always delivers true. Given that it all works on GitLab and locally,
                // we must assume that there is a difference between glib 2.56/gtk 3.22
                // on Bionic on Travis-CI and glib 2.58/gtk 3.24 on Buster on GitLab
                // that explains this. For now use a sledgehammer approach on all
                // platforms.
                /*
                glib::idle_add_local({
                    let aa = a.clone();
                    move || {

                        println!("ZZZZ  Attempt to quit.");

                        if glib::MainContext::default().pending() {
                            Continue(true)
                        } else {

                            println!("ZZZZ  Quitting.");

                            aa.quit();
                            Continue(false)
                        }
                    }
                });
                 */
                glib::timeout_add_seconds_local(1, {
                    let aa = a.clone();
                    move || {
                        aa.quit();
                        Continue(false)
                    }
                });
            }
        });
    });
    application.connect_activate(|_|{}); // Avoids a warning.
    application.run(&[]);
}
