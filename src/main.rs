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

use gio;
use gio::prelude::*;
use gtk;
// use gtk::prelude::*;

mod about;
mod arcam_protocol;
mod comms_manager;
mod control_window;
mod functionality;
mod socket_support;

#[cfg(not(test))]
fn main() {
    let application = gtk::Application::new(Some("uk.org.russel.arcamclient"), gio::ApplicationFlags::empty()).expect("Application creation failed");
    glib::set_application_name("ArcamClient");
    application.connect_startup(move |app| {
        let _control_window = control_window::ControlWindow::new(&app, None);
    });
    // Get a glib-gio warning if activate is not handled.
    application.connect_activate(move |_| { });
    application.run(&[]);
}
