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

use arcamclient::arcam_protocol::{AnswerCode, Command, Request, ZoneNumber};
use arcamclient::comms_manager;
use arcamclient::control_window;
use arcamclient::functionality;

#[test]
fn ui_test() {
    let application = gtk::Application::new(Some("uk.org.winder.arcamclient.ui_test"), gio::ApplicationFlags::empty()).unwrap();
    application.connect_startup(move |app| {
        let control_window = control_window::ControlWindow::new(&app, None);
        // Attempt a connection to somewhere guaranteed to fail. Use 127.0.0.2 because
        // it is not 127.0.0.1 and yet is a loopback address. This ensures the UI state
        // initialisation required with no attempt to use a mock AVR850.
        control_window.set_address("127.0.0.2");
        control_window.get_connect_chooser().set_active(true); // Won't se the state, so…
        control_window.get_connect_display().set_text("Connected"); // …set it manually.
        // Amend the state of the UI. Replace the channel to the comms manager with one
        // that we can use for checking the packets sent. This cuts off the comms manager
        // so that it's state no longer matters for the tests.
        let (tx_queue, mut rx_queue) = futures::channel::mpsc::channel::<Vec<u8>>(10);
        control_window.get_to_comms_manager_field().borrow_mut().replace(tx_queue);
        // Set some tests going.
        glib::MainContext::default().spawn_local({
            let a = app.clone();
            let c_w = control_window.clone();
            async move {

                eprintln!("ui_test::ui_test: set Zone 1 volume");
                // This should trigger a change to an volume ScrollButton and therefore
                // send a message to the amp.
                c_w.set_volume_chooser(ZoneNumber::One, 20.0);

                eprintln!("ui_test::ui_test: await packet on queue of packet to comms_manager.");
                match rx_queue.next().await {
                    Some(s) => assert_eq!(s, Request::new(ZoneNumber::One, Command::SetRequestVolume, vec![0x14]).unwrap().to_bytes()),
                    None => assert!(false, "Failed to get a value from the response queue."),
                };

                eprintln!("ui_test::ui_test: set up application termination.");
                glib::source::timeout_add_seconds_local(1, {
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
    eprintln!("ui_test::ui_test: starting the application event loop.");
    application.run(&[]);
    eprintln!("ui_test::ui_test: the application event loop has terminated.");
}
