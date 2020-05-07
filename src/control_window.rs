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

use std::borrow::BorrowMut; // Is this actually used?
use std::cell::RefCell;
use std::rc::Rc;

use gio;
use gio::prelude::*;
use glib;
//use glib::prelude::*;
use gtk;
use gtk::prelude::*;

//use futures;

use num_derive::FromPrimitive;  // Apparently unused, but it is necessary.
use num_traits::FromPrimitive;

use crate::about;
use crate::functionality;
use crate::arcam_protocol::{Brightness, Source, ZoneNumber};

pub struct ControlWindow {
    window: gtk::ApplicationWindow,
    address: gtk::Entry,
    connect_display: gtk::Label,
    connect_chooser: gtk::CheckButton,
    source_display: gtk::Label,
    source_chooser: gtk::ComboBoxText,
    brightness_display: gtk::Label,
    brightness_chooser: gtk::ComboBoxText,
    zone_1_volume_display: gtk::Label,
    zone_1_volume_chooser: gtk::SpinButton,
    zone_1_mute_display: gtk::Label,
    zone_1_mute_chooser: gtk::CheckButton,
    zone_2_volume_display: gtk::Label,
    zone_2_volume_chooser: gtk::SpinButton,
    zone_2_mute_display: gtk::Label,
    zone_2_mute_chooser: gtk::CheckButton,
    radio_data: gtk::Box,
    radio_station_display: gtk::Label,
    music_type_display: gtk::Label,
    rds_dls: gtk::Label,
    to_comms_manager: RefCell<Option<futures::channel::mpsc::Sender<Vec<u8>>>>,
}

impl ControlWindow {
    pub fn new(application: &gtk::Application, port_number: Option<u16>) -> Rc<Self> {
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
        header_bar.show();
        let menu_button = gtk::MenuButton::new();
        menu_button.set_image(Some(&gtk::Image::new_from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::Button.into())));
        menu_button.show();
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
        window.show();
        let address: gtk::Entry = builder.get_object("address").unwrap();
        let connect_display: gtk::Label = builder.get_object("connect_display").unwrap();
        let connect_chooser: gtk::CheckButton = builder.get_object("connect_chooser").unwrap();
        let source_display: gtk::Label = builder.get_object("source_display").unwrap();
        let source_chooser: gtk::ComboBoxText= builder.get_object("source_chooser").unwrap();
        let brightness_display: gtk::Label = builder.get_object("brightness_display").unwrap();
        let brightness_chooser: gtk::ComboBoxText = builder.get_object("brightness_chooser").unwrap();
        let zone_1_volume_display: gtk::Label = builder.get_object("zone_1_volume_display").unwrap();
        let zone_1_volume_chooser: gtk::SpinButton = builder.get_object("zone_1_volume_chooser").unwrap();
        let zone_1_mute_display: gtk::Label = builder.get_object("zone_1_mute_display").unwrap();
        let zone_1_mute_chooser: gtk::CheckButton = builder.get_object("zone_1_mute_chooser").unwrap();
        let zone_2_volume_display: gtk::Label = builder.get_object("zone_2_volume_display").unwrap();
        let zone_2_volume_chooser: gtk::SpinButton = builder.get_object("zone_2_volume_chooser").unwrap();
        let zone_2_mute_display: gtk::Label = builder.get_object("zone_2_mute_display").unwrap();
        let zone_2_mute_chooser: gtk::CheckButton = builder.get_object("zone_2_mute_chooser").unwrap();
        let radio_data: gtk::Box = builder.get_object("radio_data").unwrap();
        let radio_station_display: gtk::Label = builder.get_object("radio_station_display").unwrap();
        let music_type_display: gtk::Label = builder.get_object("music_type_display").unwrap();
        let rds_dls: gtk::Label = builder.get_object("RDS_DLS").unwrap();
        let control_window = Rc::new(ControlWindow {
            window,
            address,
            connect_display,
            connect_chooser,
            source_display,
            source_chooser,
            brightness_display,
            brightness_chooser,
            zone_1_volume_display,
            zone_1_volume_chooser,
            zone_1_mute_display,
            zone_1_mute_chooser,
            zone_2_volume_display,
            zone_2_volume_chooser,
            zone_2_mute_display,
            zone_2_mute_chooser,
            radio_data,
            radio_station_display,
            music_type_display,
            rds_dls,
            to_comms_manager: RefCell::new(None),
        });
        let (tx_from_comms_manager, rx_from_comms_manager) = glib::MainContext::channel(glib::source::PRIORITY_DEFAULT);
        rx_from_comms_manager.attach(None, {
            let c_w = control_window.clone();
            let mut queue = vec![];
            move |datum: Vec<u8>| {  //  TODO Why is this type specification required?
                for c in datum.iter() {
                    queue.push(*c);
                }
                while functionality::try_parse_of_response_data(&c_w, &mut queue) {
                    eprintln!("control_window::rx_from_comms_manager listener: got a good packet.");
                }
                Continue(true)
            }
        });
        control_window.connect_chooser.connect_toggled({
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
                                let p_n = match port_number {
                                    Some(p) => p,
                                    None => 50000
                                };
                                eprintln!("control_window::connect_toggled: connect to {}:{}", &address, p_n);
                                match functionality::connect_to_amp(
                                    &tx_from_comms_manager,
                                    &address.to_string(),
                                    p_n,
                                ) {
                                    Ok(s) => {
                                        //  TODO How come a mutable borrow works here?
                                        //  TODO Why is the argument to replace here not an Option?
                                        c_w.to_comms_manager.borrow_mut().replace(s);
                                        eprintln!("control_window::connect_toggled: connected to amp at {}:{}", address, p_n);
                                    },
                                    Err(e) => eprintln!("control_window::connect_toggled: failed to connect to amp – {:?}", e),
                                };
                                functionality::initialise_control_window(&mut c_w.get_to_comms_manager());
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
                    eprintln!("control_window::connect_toggled: terminate connection to amp.");
                    functionality::disconnect_from_amp();
                }
            }
        });
        control_window.zone_1_volume_chooser.connect_changed({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_volume_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::One, button.get_value());
                }
            }
        });
        control_window.zone_1_mute_chooser.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_mute_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::One, button.get_active())
                }
            }
        });
        control_window.zone_2_volume_chooser.connect_changed({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_volume_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::Two, button.get_value());
                }
            }
        });
        control_window.zone_2_mute_chooser.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_mute_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::Two, button.get_active())
                }
            }
        });
        control_window
    }

    fn get_to_comms_manager(self: &Self) -> futures::channel::mpsc::Sender<Vec<u8>> {
        self.to_comms_manager.borrow().as_ref().unwrap().clone()
    }

    pub fn set_connect_display(self: &Self, on: bool) {
        self.connect_display.set_text(if on { "Connected" } else { "Not connected" });
    }

    pub fn set_source_display(self: &Self, source: Source) {
        self.source_display.set_text(&format!("{:?}", source));
        if source == Source::TUNER { self.radio_data.show(); }
        else { self.radio_data.hide(); }
        self.set_source_chooser(source);
    }

    pub fn set_source_chooser(self: &Self, source: Source) {
        self.source_chooser.set_active_id(Some(&format!("{:?}", source)));
    }

    pub fn set_brightness_display(self: &Self, level: Brightness) {
        let brightness_id= format!("{:?}", level);
        self.brightness_display.set_text(&brightness_id);
        self.set_brightness_chooser(level);
    }

    pub fn set_brightness_chooser(self: &Self, level: Brightness) {
        let brightness_id= format!("{:?}", level);
        self.brightness_chooser.set_active_id(Some(&brightness_id));
    }

    pub fn set_mute_display(self: &Self, zone: ZoneNumber, on_off: bool) {
        let text = if on_off { "On" } else { "Muted" };
        match zone {
            ZoneNumber::One => self.zone_1_mute_display.set_text(text),
            ZoneNumber::Two => self.zone_2_mute_display.set_text(text),
        }
    }

    pub fn set_mute_chooser(self: &Self, zone: ZoneNumber, on_off: bool) {
        match zone {
            ZoneNumber::One => self.zone_1_mute_chooser.set_mode(on_off),
            ZoneNumber::Two => self.zone_2_mute_chooser.set_mode(on_off),
        }
    }

    pub fn set_volume_display(self: &Self, zone: ZoneNumber, volume: f64) {
        assert!(volume < 100.0);
        let text = volume.to_string();
        match zone {
            ZoneNumber::One => self.zone_1_volume_display.set_text(&text),
            ZoneNumber::Two => self.zone_2_volume_display.set_text(&text),
        }
    }

    pub fn set_volume_chooser(self: &Self, zone: ZoneNumber, volume: f64) {
        assert!(volume < 100.0);
        match zone {
            ZoneNumber::One => self.zone_1_volume_chooser.set_value(volume),
            ZoneNumber::Two => self.zone_2_volume_chooser.set_value(volume),
        }
    }

    pub fn set_radio_station_display(self: &Self, station: &str) {
        self.radio_station_display.set_text(station);
        self.radio_data.show();
    }

    pub fn set_music_type_display(self: &Self, style: &str) {
        self.music_type_display.set_text(style);
        self.radio_data.show();
    }

    pub fn set_rds_dls(self: &Self, text: &str) {
        self.rds_dls.set_text(text);
        self.radio_data.show();
    }

    pub fn get_connect_display_value(self: &Self) -> bool {
        match self.connect_display.get_text().unwrap().as_str() {
            "Connected" => true,
            "Not connected" => false,
            x => panic!("Illegal value in connect_display – {}", x),
        }
    }

    pub fn get_connect_chooser_value(self: &Self) -> bool {
        self.connect_chooser.get_active() as bool
    }

    pub fn get_brightness_display_value(self: &Self) -> Brightness {
        match self.brightness_display.get_text().unwrap().as_str() {
            "Off" => Brightness::Off,
            "Level1" => Brightness::Level1,
            "Level2" => Brightness::Level2,
            x => panic!("Illegal brightness value from display – {}", x),
        }
    }

    pub fn get_volume_display_value(self: &Self, zone: ZoneNumber) -> u8 {
        match match zone {
            ZoneNumber::One => self.zone_1_volume_display.get_text(),
            ZoneNumber::Two => self.zone_2_volume_display.get_text(),
        } {
            Some(s) => match s.parse::<u8>() {
                Ok(v) => v,
                Err(e) => 0u8,
            },
            None => 0u8,
        }
    }

    pub fn get_mute_display_value(self: &Self, zone: ZoneNumber) -> bool {
        match match zone {
            ZoneNumber::One => self.zone_1_mute_display.get_text(),
            ZoneNumber::Two => self.zone_2_mute_display.get_text(),
        } {
            Some(s) => match s.as_str() {
                "Muted" => false,
                "On" => true,
                x => panic!("Illegal value for muted – {}", x),
            },
            None => true,
        }
    }

    fn is_connected(self: &Self) -> bool {
        if self.get_connect_display_value() {
            true
        } else {
            if ! cfg!(test) {
                let dialogue = gtk::MessageDialog::new(
                    Some(&self.window),
                    gtk::DialogFlags::MODAL,
                    gtk::MessageType::Info,
                    gtk::ButtonsType::Ok,
                    "Not connected to an amplifier."
                );
                dialogue.run();
                dialogue.destroy();
            }
            false
        }
    }

    //  Some methods needed for the integration tests.

    //  ui_test has to connect to a mock amp but without a mock amp.
    pub fn get_connect_display(self: &Self) -> gtk::Label { self.connect_display.clone() }

    //  system_test and ui_test have to connect to a mock amp.
    pub fn get_connect_chooser(self: &Self) -> gtk::CheckButton { self.connect_chooser.clone() }

    //  system_test and ui_test have to set the address.
    pub fn set_address(self: &Self, address: &str) { self.address.set_text(address); }

    //  ui_test has to hack the state.
    pub fn get_to_comms_manager_field(self: &Self) -> &RefCell<Option<futures::channel::mpsc::Sender<Vec<u8>>>> { &self.to_comms_manager }

}
