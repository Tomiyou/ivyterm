use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::{gdk::RGBA, pango::FontDescription};

use super::IvyApplication;

impl IvyApplication {
    pub fn update_foreground_color(&self, rgba: RGBA) {
        let mut config = self.imp().config.borrow_mut();
        let [_foreground, background] = config.main_colors;
        config.main_colors = [rgba, background];
        drop(config);

        self.reload_css_colors();
    }

    pub fn update_background_color(&self, rgba: RGBA) {
        let mut config = self.imp().config.borrow_mut();
        let [foreground, _background] = config.main_colors;
        config.main_colors = [foreground, rgba];
        drop(config);

        self.reload_css_colors();
    }

    pub fn update_font(&self, font_desc: FontDescription) {
        let mut config = self.imp().config.borrow_mut();
        config.font_desc = font_desc;
        drop(config);

        self.refresh_terminals();
    }
}
