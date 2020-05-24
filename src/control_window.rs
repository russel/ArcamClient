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

//! This module provides all the structs, enums and functions associated with display and
//! control of the UI.

use std::borrow::BorrowMut; // Is this actually used?
use std::cell::RefCell;
use std::rc::Rc;

use gio;
use gio::prelude::*;
use glib;
//use glib::prelude::*;
use gtk;
use gtk::prelude::*;

use log::debug;

use num_derive::FromPrimitive;  // Apparently unused, but it is necessary.
use num_traits::FromPrimitive;

use crate::about;
use crate::functionality;
use crate::arcam_protocol::{Brightness, MuteState, PowerState, Source, ZoneNumber};

/// An analogue to bool that tries to avoid any spelling errors
/// in the strings used as representation – needed for the UI.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ConnectedState {
    Connected,
    NotConnected,
}

impl ToString for ConnectedState {
    fn to_string(&self) -> String {
        match self {
            Self::Connected => "Connected".to_string(),
            Self::NotConnected => "Not Connected".to_string(),
        }
    }
}

impl From<&str> for ConnectedState {
    fn from(s: &str) -> Self {
        match s {
            "Connected" => Self::Connected,
            "Not Connected" => Self::NotConnected,
            x => panic!("Illegal value for ConnectedState, {}", x),
        }
    }
}

impl From<bool> for ConnectedState {
    fn from(b: bool) -> Self {
        match b {
            true => Self::Connected,
            false => Self::NotConnected,
        }
    }
}

impl From<ConnectedState> for bool {
    fn from(c: ConnectedState) -> Self {
        match c {
            ConnectedState::Connected => true,
            ConnectedState::NotConnected => false,
        }
    }
}

/// The struct holding all the "handles" to the UI components.
///
/// Some of the components are just display areas for showing the current state
/// of the amplifier, some components are controllers for causing data to be sent
/// to the amplifier to change the state.
///
/// This struct also keeps track of the send end of the channel down which to send
/// data to be forwarded to the amplifier.
pub struct ControlWindow {
    window: gtk::ApplicationWindow,
    address: gtk::Entry,
    connect_display: gtk::Label,
    connect_chooser: gtk::CheckButton,
    brightness_display: gtk::Label,
    brightness_chooser: gtk::ComboBoxText,
    zone_1_power_display: gtk::Label,
    zone_1_power_chooser: gtk::CheckButton,
    zone_1_volume_display: gtk::Label,
    zone_1_volume_chooser: gtk::SpinButton,
    zone_1_mute_display: gtk::Label,
    zone_1_mute_chooser: gtk::CheckButton,
    zone_1_source_display: gtk::Label,
    zone_1_source_chooser: gtk::ComboBoxText,
    zone_1_radio_data: gtk::Box,
    zone_1_radio_station_display: gtk::Label,
    zone_1_music_type_display: gtk::Label,
    zone_1_dlspdt_information_display: gtk::Label,
    zone_2_power_display: gtk::Label,
    zone_2_power_chooser: gtk::CheckButton,
    zone_2_volume_display: gtk::Label,
    zone_2_volume_chooser: gtk::SpinButton,
    zone_2_mute_display: gtk::Label,
    zone_2_mute_chooser: gtk::CheckButton,
    zone_2_source_display: gtk::Label,
    zone_2_source_chooser: gtk::ComboBoxText,
    zone_2_radio_data: gtk::Box,
    zone_2_radio_station_display: gtk::Label,
    zone_2_music_type_display: gtk::Label,
    zone_2_dlspdt_information_display: gtk::Label,
    to_comms_manager: RefCell<Option<futures::channel::mpsc::Sender<Vec<u8>>>>,
}

impl ControlWindow {
    /// Create a new instance.
    ///
    /// Reads the Glade file, picks out all the UI bits there are "handles" for and
    /// sets up all the event handlers for the events associates with the control
    /// UI components.
    pub fn new(application: &gtk::Application, port_number: Option<u16>) -> Rc<Self> {
        let builder = gtk::Builder::new_from_string(include_str!("resources/arcamclient.glade"));
        let window: gtk::ApplicationWindow = builder.get_object("application_window").unwrap();
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
        let brightness_display: gtk::Label = builder.get_object("brightness_display").unwrap();
        let brightness_chooser: gtk::ComboBoxText = builder.get_object("brightness_chooser").unwrap();
        let zone_1_power_display: gtk::Label = builder.get_object("zone_1_power_display").unwrap();
        let zone_1_power_chooser: gtk::CheckButton = builder.get_object("zone_1_power_chooser").unwrap();
        let zone_1_volume_display: gtk::Label = builder.get_object("zone_1_volume_display").unwrap();
        let zone_1_volume_chooser: gtk::SpinButton = builder.get_object("zone_1_volume_chooser").unwrap();
        let zone_1_mute_display: gtk::Label = builder.get_object("zone_1_mute_display").unwrap();
        let zone_1_mute_chooser: gtk::CheckButton = builder.get_object("zone_1_mute_chooser").unwrap();
        let zone_1_source_display: gtk::Label = builder.get_object("zone_1_source_display").unwrap();
        let zone_1_source_chooser: gtk::ComboBoxText= builder.get_object("zone_1_source_chooser").unwrap();
        let zone_1_radio_data: gtk::Box = builder.get_object("zone_1_radio_data").unwrap();
        let zone_1_radio_station_display: gtk::Label = builder.get_object("zone_1_radio_station_display").unwrap();
        let zone_1_music_type_display: gtk::Label = builder.get_object("zone_1_music_type_display").unwrap();
        let zone_1_dlspdt_information_display: gtk::Label = builder.get_object("zone_1_DLSPDT_information_display").unwrap();
        let zone_2_power_display: gtk::Label = builder.get_object("zone_2_power_display").unwrap();
        let zone_2_power_chooser: gtk::CheckButton = builder.get_object("zone_2_power_chooser").unwrap();
        let zone_2_volume_display: gtk::Label = builder.get_object("zone_2_volume_display").unwrap();
        let zone_2_volume_chooser: gtk::SpinButton = builder.get_object("zone_2_volume_chooser").unwrap();
        let zone_2_mute_display: gtk::Label = builder.get_object("zone_2_mute_display").unwrap();
        let zone_2_mute_chooser: gtk::CheckButton = builder.get_object("zone_2_mute_chooser").unwrap();
        let zone_2_source_display: gtk::Label = builder.get_object("zone_2_source_display").unwrap();
        let zone_2_source_chooser: gtk::ComboBoxText= builder.get_object("zone_2_source_chooser").unwrap();
        let zone_2_radio_data: gtk::Box = builder.get_object("zone_2_radio_data").unwrap();
        let zone_2_radio_station_display: gtk::Label = builder.get_object("zone_2_radio_station_display").unwrap();
        let zone_2_music_type_display: gtk::Label = builder.get_object("zone_2_music_type_display").unwrap();
        let zone_2_dlspdt_information_display: gtk::Label = builder.get_object("zone_2_DLSPDT_information_display").unwrap();
        let control_window = Rc::new(ControlWindow {
            window,
            address,
            connect_display,
            connect_chooser,
            brightness_display,
            brightness_chooser,
            zone_1_power_display,
            zone_1_power_chooser,
            zone_1_volume_display,
            zone_1_volume_chooser,
            zone_1_mute_display,
            zone_1_mute_chooser,
            zone_1_source_display,
            zone_1_source_chooser,
            zone_1_radio_data,
            zone_1_radio_station_display,
            zone_1_music_type_display,
            zone_1_dlspdt_information_display,
            zone_2_power_display,
            zone_2_power_chooser,
            zone_2_volume_display,
            zone_2_volume_chooser,
            zone_2_mute_display,
            zone_2_mute_chooser,
            zone_2_source_display,
            zone_2_source_chooser,
            zone_2_radio_data,
            zone_2_radio_station_display,
            zone_2_music_type_display,
            zone_2_dlspdt_information_display,
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
                while functionality::try_parse_of_response_data(&c_w, &mut queue) {}
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
                                debug!("Connect to {}:{}.", &address, p_n);
                                match functionality::connect_to_amp(
                                    &tx_from_comms_manager,
                                    &address.to_string(),
                                    p_n,
                                ) {
                                    Ok(s) => {
                                        //  TODO How come a mutable borrow works here?
                                        //  TODO Why is the argument to replace here not an Option?
                                        c_w.to_comms_manager.borrow_mut().replace(s);
                                        debug!("Connected to amp at {}:{}.", address, p_n);
                                    },
                                    Err(e) => debug!("Failed to connect to amp – {:?}.", e),
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
                    debug!("Terminate connection to amp.");
                    functionality::disconnect_from_amp();
                    c_w.connect_display.set_text(&ConnectedState::NotConnected.to_string());
                }
            }
        });
        control_window.zone_1_power_chooser.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_power_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::One, button.get_active().into());
                }
            }
        });
        control_window.zone_1_volume_chooser.connect_changed({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_volume_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::One, button.get_value() as u8);
                }
            }
        });
        control_window.zone_1_mute_chooser.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_mute_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::One, button.get_active().into())
                }
            }
        });
        control_window.zone_1_source_chooser.connect_changed({
            let c_w = control_window.clone();
            move |cbt| {
                if c_w.is_connected() {
                    functionality::set_source_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::One, Source::from(cbt.get_active_id().unwrap().as_ref()));
                }
            }
        });
        control_window.zone_2_power_chooser.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_power_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::Two, button.get_active().into());
                }
            }
        });
        control_window.zone_2_volume_chooser.connect_changed({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_volume_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::Two, button.get_value() as u8);
                }
            }
        });
        control_window.zone_2_mute_chooser.connect_toggled({
            let c_w = control_window.clone();
            move |button| {
                if c_w.is_connected() {
                    functionality::set_mute_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::Two, button.get_active().into())
                }
            }
        });
        control_window.zone_2_source_chooser.connect_changed({
            let c_w = control_window.clone();
            move |cbt| {
                if c_w.is_connected() {
                    functionality::set_source_on_amp(&mut c_w.get_to_comms_manager(), ZoneNumber::Two, Source::from(cbt.get_active_id().unwrap().as_ref()));
                }
            }
        });
        control_window
    }

    /// Accessor for the send end of the channel to send data to the amplifier via the comms manager.
    fn get_to_comms_manager(self: &Self) -> futures::channel::mpsc::Sender<Vec<u8>> {
        self.to_comms_manager.borrow().as_ref().unwrap().clone()
    }

    /// Sets the value shown in the connect display UI component.
    pub fn set_connect_display(self: &Self, connected: ConnectedState) {
        let string_to_set = connected.to_string();
        self.connect_display.set_text(&string_to_set);
        let value: bool = connected.into();
        if self.connect_chooser.get_active() != value {
            self.connect_chooser.set_active(value);
        }
    }

    /// Sets the value shown in the brightness display UI component.
    pub fn set_brightness_display(self: &Self, level: Brightness) {
        let brightness_id= level.to_string();
        self.brightness_display.set_text(&brightness_id);
        if self.brightness_chooser.get_active_id().unwrap() != brightness_id {
            self.brightness_chooser.set_active_id(Some(&brightness_id));
        }
    }

    /// Sets the value shown in the zone specific power display UI component.
    pub fn set_power_display(self: &Self, zone: ZoneNumber, power: PowerState) {
        let text = power.to_string();
        let (power_display, power_chooser) = match zone {
            ZoneNumber::One => (&self.zone_1_power_display, &self.zone_1_power_chooser),
            ZoneNumber::Two => (&self.zone_2_power_display, &self.zone_2_power_chooser),
        };
        power_display.set_text(&text);
        let value: bool = power.into();
        if power_chooser.get_active() != value {
            power_chooser.set_active(value);
        }
    }

    /// Sets the value shown in the zone specific volume display UI component.
    pub fn set_volume_display(self: &Self, zone: ZoneNumber, volume: u8) {
        assert!(volume < 100);
        let (volume_display, volume_chooser) = match zone {
            ZoneNumber::One => (&self.zone_1_volume_display, &self.zone_1_volume_chooser),
            ZoneNumber::Two => (&self.zone_2_volume_display, &self.zone_2_volume_chooser),
        };
        let text = volume.to_string();
        volume_display.set_text(&text);
        if volume_chooser.get_value() as u8 != volume {
            volume_chooser.set_value(volume as f64);
        }
    }

    /// Sets the value shown in the zone specific mute display UI component.
    pub fn set_mute_display(self: &Self, zone: ZoneNumber, mute: MuteState) {
        let text = mute.to_string();
        let (mute_display, mute_chooser) = match zone {
            ZoneNumber::One => (&self.zone_1_mute_display, &self.zone_1_mute_chooser),
            ZoneNumber::Two => (&self.zone_2_mute_display, &self.zone_2_mute_chooser),
        };
        mute_display.set_text(&text);
        let value: bool = mute.into();
        if mute_chooser.get_active() != value {
            mute_chooser.set_active(value);
        }
   }

    /// Sets the value shown in the zone specific source display UI component.
    pub fn set_source_display(self: &Self, zone: ZoneNumber, source: Source) {
        let (source_display, source_chooser, radio_data) = match zone {
            ZoneNumber::One => (&self.zone_1_source_display, &self.zone_1_source_chooser, &self.zone_1_radio_data),
            ZoneNumber::Two => (&self.zone_2_source_display, &self.zone_2_source_chooser, &self.zone_2_radio_data),
        };
        let source_id = format!("{:?}", source);
        source_display.set_text(&source_id);
        if source == Source::TUNER { radio_data.show(); }
        else { radio_data.hide(); }
        match source_chooser.get_active_id() {
            Some(id) => if id != source_id { source_chooser.set_active_id(Some(&source_id)); },
            None => { source_chooser.set_active_id(Some(&source_id)); },
        }
    }

    /// Sets the value in the radio station name display UI component.
    pub fn set_radio_station_display(self: &Self, zone: ZoneNumber, station: &str) {
        let (radio_data, radio_station_display) = match zone {
            ZoneNumber::One => (&self.zone_1_radio_data, &self.zone_1_radio_station_display),
            ZoneNumber::Two => (&self.zone_2_radio_data, &self.zone_2_radio_station_display),
        };
        radio_station_display.set_text(station);
        radio_data.show();
    }

    /// Sets the value in the music type display UI component.
    pub fn set_music_type_display(self: &Self, zone: ZoneNumber, style: &str) {
        let (radio_data, music_type_display) = match zone {
            ZoneNumber::One => (&self.zone_1_radio_data, &self.zone_1_music_type_display),
            ZoneNumber::Two => (&self.zone_2_radio_data, &self.zone_2_music_type_display),
        };
        music_type_display.set_text(style);
        radio_data.show();
    }

    /// Sets the value in the DLS/PDT information display UI component.
    pub fn set_dlspdt_information(self: &Self, zone:ZoneNumber, text: &str) {
        let (radio_data, dlspdt_information_display) = match zone {
            ZoneNumber::One => (&self.zone_1_radio_data, &self.zone_1_dlspdt_information_display),
            ZoneNumber::Two => (&self.zone_2_radio_data, &self.zone_2_dlspdt_information_display),
        };
        dlspdt_information_display.set_text(text);
        radio_data.show();
    }

    /// Accessor for the current value of the connect display UI component.
    pub fn get_connect_display_value(self: &Self) -> ConnectedState {
        self.connect_display.get_text().unwrap().as_str().into()
    }

    /// Accessor for the current value of the brightness display UI component.
    pub fn get_brightness_display_value(self: &Self) -> Brightness {
        self.brightness_display.get_text().unwrap().as_str().into()
    }

    /// Accessor for the current value of the zone specific power display UI component.
    pub fn get_power_display_value(self: &Self, zone: ZoneNumber) -> PowerState {
        match match zone {
            ZoneNumber::One => self.zone_1_power_display.get_text(),
            ZoneNumber::Two => self.zone_2_power_display.get_text(),
        } {
            Some(s) => s.as_str().into(),
            None => panic!("Could not get UI power status for zone {:?}", zone),
        }
    }

    /// Accessor for the current value of the zone specific volume display UI component.
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

    /// Accessor for the current value of the zone specific mute display UI component.
    pub fn get_mute_display_value(self: &Self, zone: ZoneNumber) -> MuteState {
        match match zone {
            ZoneNumber::One => self.zone_1_mute_display.get_text(),
            ZoneNumber::Two => self.zone_2_mute_display.get_text(),
        } {
            Some(s) => s.as_str().into(),
            None => panic!("Could not get UI mute status for zone {:?}", zone),
        }
    }

    /// Accessor for the current value of the zone specific source display UI component.
    pub fn get_source_display_value(self: &Self, zone: ZoneNumber) -> Source {
        let source_display= match zone {
            ZoneNumber::One => &self.zone_1_source_display,
            ZoneNumber::Two => &self.zone_2_source_display,
        };
        source_display.get_text().unwrap().as_str().into()
    }

    /// Accessor for whether the client is connected to an amplifier – real or mock.
    ///
    /// In non-test situation, if there is no connection, a message dialogue is displayed.
    fn is_connected(self: &Self) -> bool {
        let rc: bool = self.get_connect_display_value().into();
        if !rc {
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
        }
        rc
    }

    // Some methods needed for the integration and system tests that break the
    // overall abstraction.

    #[doc(hidden)]
    pub fn get_to_comms_manager_field(self: &Self) -> &RefCell<Option<futures::channel::mpsc::Sender<Vec<u8>>>> {
        &self.to_comms_manager
    }

    #[doc(hidden)]
    pub fn set_address(self: &Self, address: &str) {
        self.address.set_text(address);
    }

    #[doc(hidden)]
    pub fn set_connect_chooser(self: &Self, connect: bool) {
        self.connect_chooser.set_active(connect)
    }

    #[doc(hidden)]
    pub fn set_power_chooser(self: &Self, zone: ZoneNumber, power: PowerState) {
        match zone {
            ZoneNumber::One => &self.zone_1_power_chooser,
            ZoneNumber::Two => &self.zone_2_power_chooser,
        }.set_active(power.into());
    }

    #[doc(hidden)]
    pub fn set_volume_chooser(self: &Self, zone: ZoneNumber, volume: f64) {
        match zone {
            ZoneNumber::One => &self.zone_1_volume_chooser,
            ZoneNumber::Two => &self.zone_2_volume_chooser,
        }.set_value(volume);
    }

    #[doc(hidden)]
    pub fn set_mute_chooser(self: &Self, zone: ZoneNumber, mute: MuteState) {
        match zone {
            ZoneNumber::One => &self.zone_1_mute_chooser,
            ZoneNumber::Two => &self.zone_2_mute_chooser,
        }.set_active(mute.into());
    }

    #[doc(hidden)]
    pub fn set_source_chooser(self: &Self, zone: ZoneNumber, source: Source) {
        match zone {
            ZoneNumber::One => &self.zone_1_source_chooser,
            ZoneNumber::Two => &self.zone_2_source_chooser,
        }.set_active_id(Some(&format!("{:?}", source)));
    }

}
