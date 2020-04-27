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

use std::rc::Rc;

use gio;
use gio::prelude::*;
//use glib;
//use glib::prelude::*;
use gtk;
use gtk::prelude::*;

use crate::about;
use crate::functionality;

pub struct ControlWindow {
    window: gtk::ApplicationWindow,
    brightness: gtk::Label,
    zone_1_adjustment: gtk::Adjustment,
    zone_1_mute: gtk::CheckButton,
    zone_2_adjustment: gtk::Adjustment,
    zone_2_mute: gtk::CheckButton,
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
        let brightness: gtk::Label = builder.get_object::<gtk::Label>("brightness").unwrap();
        let zone_1_adjustment: gtk::Adjustment = builder.get_object("zone1_adjustment").unwrap();
        let zone_1_mute: gtk::CheckButton = builder.get_object("zone1_mute").unwrap();
        let zone_2_adjustment: gtk::Adjustment = builder.get_object("zone2_adjustment").unwrap();
        let zone_2_mute: gtk::CheckButton = builder.get_object("zone2_mute").unwrap();
        let menu_builder = gtk::Builder::new_from_string(include_str!("resources/application_menu.xml"));
        let application_menu: gio::Menu = menu_builder.get_object("application_menu").unwrap();
        let about_action = gio::SimpleAction::new("about", None);
        window.add_action(&about_action);
        menu_button.set_menu_model(Some(&application_menu));
        header_bar.pack_end(&menu_button);
        window.set_titlebar(Some(&header_bar));
        window.show_all();
        let control_window = Rc::new(ControlWindow {
            window,
            brightness,
            zone_1_adjustment,
            zone_1_mute,
            zone_2_adjustment,
            zone_2_mute,
        });
        about_action.connect_activate({
            let c_w = control_window.clone();
            move |_, _| about::present(Some(&c_w.window))
        });
        control_window.initialise_control_window();
        control_window
    }

    fn initialise_control_window(self: &Self) {
        functionality::get_brightness_from_amp();
        functionality::get_zone_1_volume_from_amp();
        functionality::get_zone_1_mute_from_amp();
        functionality::get_zone_2_volume_from_amp();
        functionality::get_zone_2_mute_from_amp();
    }

    pub fn set_brightness(self: &Self, level: u8) {
        assert!(level < 4);
        let mut brightness_label= if level == 0 { "Off".to_string() } else { "Level ".to_string() + &level.to_string() };
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
