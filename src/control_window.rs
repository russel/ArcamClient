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
use glib;
//use glib::prelude::*;
use gtk;
use gtk::prelude::*;

//use futures;

use crate::about;
use crate::functionality;
use crate::arcam_protocol::{Source, ZoneNumber};
use std::borrow::BorrowMut;

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
    to_comms_manager: RefCell<Option<futures::channel::mpsc::Sender<Vec<u8>>>>,
}

impl ControlWindow {
    pub fn new(application: &gtk::Application) -> Rc<Self> {
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
                functionality::try_parse_of_response_data(&c_w, &mut queue);
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
                                eprintln!("control_window::connect_toggled: connect to {}:50000", &address);
                                match functionality::connect_to_amp(
                                    &tx_from_comms_manager,
                                    &address.to_string(),
                                    50000,
                                ) {
                                    Ok(s) => {
                                        //  TODO How come a mutable borrow works here?
                                        //  TODO Why is the argument to replace here not an Option?
                                        c_w.to_comms_manager.borrow_mut().replace(s);
                                        eprintln!("control_window::connect_toggled: connected to amp at {}:50000", address);
                                    },
                                    Err(e) => eprintln!("control_window::connect_toggled: failed to connect to amp – {:?}", e),
                                };
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
                    eprintln!("control_window::connect_toggled: terminate connection to amp.");
                    functionality::disconnect_from_amp();
                }
            }
        });
        control_window.zone_1_volume_chooser.connect_changed({
            let c_w = control_window.clone();
            move |button| {
                functionality::set_volume_on_amp(&c_w, ZoneNumber::One, button.get_value());
            }
        });
        control_window.zone_1_mute_chooser.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                functionality::set_mute_on_amp(&c_w, ZoneNumber::One, button.get_active())
            }
        });
        control_window.zone_2_volume_chooser.connect_changed({
            let c_w = control_window.clone();
            move |button| {
                functionality::set_volume_on_amp(&c_w, ZoneNumber::Two, button.get_value());
            }
        });
        control_window.zone_2_mute_chooser.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                functionality::set_mute_on_amp(&c_w, ZoneNumber::Two, button.get_active())
            }
        });
        control_window
    }

    pub fn set_source_display(self: &Self, source: Source) {
        self.source_display.set_text(&format!("{:?}", source));
    }

    pub fn set_brightness_display(self: &Self, level: u8) {
        assert!(level < 3);
        let brightness_id= if level == 0 { "Off".to_string() } else { "Level_".to_string() + &level.to_string() };
        self.brightness_display.set_text(&brightness_id);
    }

    pub fn set_brightness_chooser(self: &Self, level: u8) {
        assert!(level < 3);
        let brightness_id= if level == 0 { "Off".to_string() } else { "Level_".to_string() + &level.to_string() };
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

    pub fn get_application(self: &Self) -> Option<gtk::Application> { self.window.get_application() }

    pub fn get_window(self: &Self) -> gtk::ApplicationWindow { self.window.clone() }

    pub fn get_connect(self: &Self) -> gtk::CheckButton { self.connect_chooser.clone() }

    pub fn get_to_comms_manager(self: &Self) -> &RefCell<Option<futures::channel::mpsc::Sender<Vec<u8>>>> { &self.to_comms_manager }

    //  Some methods needed for the integration tests.

    pub fn set_address(self: &Self, address: &str) {
        self.address.set_text(address);
    }

    pub fn create_dummy_control_window_for_testing(application: &gtk::Application) -> Self {
        let zone_1_adjustment = gtk::Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 10.0);
        let zone_2_adjustment = gtk::Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 10.0);
        ControlWindow {
            window: gtk::ApplicationWindow::new(application),
            address: gtk::Entry::new(),
            connect_display: gtk::Label::new(None),
            connect_chooser: gtk::CheckButton::new(),
            source_display: gtk::Label::new(None),
            source_chooser: gtk::ComboBoxText::new(),
            brightness_display: gtk::Label::new(None),
            brightness_chooser: gtk::ComboBoxText::new(),
            zone_1_volume_display: gtk::Label::new(None),
            zone_1_volume_chooser: gtk::SpinButton::new(Some(&zone_1_adjustment), 1.0, 3),
            zone_1_mute_display: gtk::Label::new(None),
            zone_1_mute_chooser: gtk::CheckButton::new(),
            zone_2_volume_display: gtk::Label::new(None),
            zone_2_volume_chooser: gtk::SpinButton::new(Some(&zone_2_adjustment), 1.0, 3),
            zone_2_mute_display: gtk::Label::new(None),
            zone_2_mute_chooser: gtk::CheckButton::new(),
            to_comms_manager: RefCell::new(None)
        }
    }
}
