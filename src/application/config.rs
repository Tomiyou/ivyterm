use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::{gdk::RGBA, pango::FontDescription};

use crate::settings::{IvyColor, IvyFont};

use super::IvyApplication;

impl IvyApplication {
    pub fn update_foreground_color(&self, rgba: RGBA) {
        let mut config = self.imp().config.borrow_mut();
        let color: IvyColor = rgba.into();
        config.foreground = color;
        drop(config);

        self.reload_css_colors();
    }

    pub fn update_background_color(&self, rgba: RGBA) {
        let mut config = self.imp().config.borrow_mut();
        let color: IvyColor = rgba.into();
        config.background = color;
        drop(config);

        self.reload_css_colors();
    }

    pub fn update_font(&self, font_desc: FontDescription) {
        let mut config = self.imp().config.borrow_mut();
        let font: IvyFont = font_desc.into();
        config.font = font;
        drop(config);

        self.refresh_terminals();
    }
}
