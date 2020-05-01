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

use std::cell::RefCell;
use std::rc::Rc;

use gio;
use gio::prelude::*;
//use glib;
//use glib::prelude::*;
use gtk;
use gtk::prelude::*;

use crate::about;
use crate::comms_manager;
use crate::functionality;

// Integration tests require all fields to be public.
pub struct ControlWindow {
    pub window: gtk::ApplicationWindow,
    pub address: gtk::Entry,
    pub connect: gtk::CheckButton, // Access required in comms_manager.
    pub brightness: gtk::Label,
    pub zone_1_adjustment: gtk::Adjustment,
    pub zone_1_mute: gtk::CheckButton,
    pub zone_2_adjustment: gtk::Adjustment,
    pub zone_2_mute: gtk::CheckButton,
    pub socket_connection: RefCell<Option<comms_manager::SocketConnection>>, // Access required in functionality and comms_manager,
}

impl ControlWindow {
    pub fn new(application: &gtk::Application) -> Rc<ControlWindow> {
        let builder = gtk::Builder::new_from_string(include_str!("resources/arcamclient.glade"));
        let window: gtk::ApplicationWindow = builder.get_object("applicationWindow").unwrap();
        window.set_application(Some(application));
        window.connect_delete_event({
            let a = application.clone();
            move |_, _| {
                a.quit();
                Inhibit(false)
            }
        });
        let header_bar = gtk::HeaderBar::new();
        header_bar.set_title(Some("ArcamClient"));
        header_bar.set_show_close_button(true);
        let menu_button = gtk::MenuButton::new();
        menu_button.set_image(Some(&gtk::Image::new_from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::Button.into())));
        let address: gtk::Entry = builder.get_object("address").unwrap();
        let connect: gtk::CheckButton = builder.get_object("connect").unwrap();
        let brightness: gtk::Label = builder.get_object("brightness").unwrap();
        let zone_1_adjustment: gtk::Adjustment = builder.get_object("zone_1_adjustment").unwrap();
        let zone_1_mute: gtk::CheckButton = builder.get_object("zone_1_mute").unwrap();
        let zone_2_adjustment: gtk::Adjustment = builder.get_object("zone_2_adjustment").unwrap();
        let zone_2_mute: gtk::CheckButton = builder.get_object("zone_2_mute").unwrap();
        let menu_builder = gtk::Builder::new_from_string(include_str!("resources/application_menu.xml"));
        let application_menu: gio::Menu = menu_builder.get_object("application_menu").unwrap();
        let about_action = gio::SimpleAction::new("about", None);
        about_action.connect_activate({
            let w = window.clone();
            move |_, _| about::present(Some(&w))
        });
        window.add_action(&about_action);
        menu_button.set_menu_model(Some(&application_menu));
        header_bar.pack_end(&menu_button);
        window.set_titlebar(Some(&header_bar));
        window.show_all();
        let control_window = Rc::new(ControlWindow {
            window,
            address,
            connect,
            brightness,
            zone_1_adjustment,
            zone_1_mute,
            zone_2_adjustment,
            zone_2_mute,
            socket_connection: RefCell::new(None),
        });
        control_window.connect.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                // NB this is the state after the UI activity that caused the event that called the closure.
                if button.get_active() {
                    match c_w.address.get_text() {
                        Some(address) => {
                            if address.len() == 0 {
                                let dialogue = gtk::MessageDialog::new(
                                    Some(&c_w.window),
                                    gtk::DialogFlags::MODAL,
                                    gtk::MessageType::Info,
                                    gtk::ButtonsType::Ok,
                                    "Empty string as address, not connecting.",
                                );
                                dialogue.run();
                                dialogue.destroy();
                                button.set_active(false);
                            } else {
                                let address = address;
                                eprintln!("Connect to {}:50000", &address);
                                glib::MainContext::default().spawn_local(
                                    comms_manager::initialise_socket_and_listen_for_packets_from_amp(
                                        c_w.clone(), address.to_string(), 50000));
                                functionality::initialise_control_window(&c_w);
                            }
                        }
                        None => {
                            let dialogue = gtk::MessageDialog::new(
                                Some(&c_w.window),
                                gtk::DialogFlags::MODAL,
                                gtk::MessageType::Info,
                                gtk::ButtonsType::Ok,
                                "No address to connect to."
                            );
                            dialogue.run();
                            dialogue.destroy();
                            button.set_active(false);
                        },
                    };
                } else {
                    eprintln!("Terminate connection to amp.");
                    glib::MainContext::default().spawn_local(comms_manager::terminate_connection(c_w.clone()));
                }
            }
        });
        control_window.zone_1_mute.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                functionality::set_zone_1_mute_on_amp(&c_w, button.get_active())
            }
        });
        control_window.zone_2_mute.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                functionality::set_zone_2_mute_on_amp(&c_w, button.get_active())
            }
        });
        control_window
    }

    pub fn set_brightness(self: &Self, level: u8) {
        assert!(level < 4);
        let brightness_label= if level == 0 { "Off".to_string() } else { "Level ".to_string() + &level.to_string() };
        self.brightness.set_text(&brightness_label);
    }

    pub fn set_zone_1_mute(self: &Self, on_off: u8) {
        assert!(on_off < 2);
        if on_off == 0 { self.zone_1_mute.set_mode(false); }
        else { self.zone_1_mute.set_mode(true); }
    }

    pub fn set_zone_1_volume(self: &Self, volume: u8) {
        assert!(volume < 100);
        self.zone_1_adjustment.set_value(volume as f64);
    }

    pub fn set_zone_2_mute(self: &Self, on_off: u8) {
        assert!(on_off < 2);
        if on_off == 0 { self.zone_2_mute.set_mode(false); }
        else { self.zone_2_mute.set_mode(true); }
    }

    pub fn set_zone_2_volume(self: &Self, volume: u8) {
        assert!(volume < 100);
        self.zone_2_adjustment.set_value(volume as f64);
    }

}
