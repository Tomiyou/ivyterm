use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::{gdk::{Event, RGBA}, pango::FontDescription, ShortcutTrigger};

use crate::{keyboard::{check_keybinding_match, Keybinding, KeyboardAction}, settings::{IvyColor, IvyFont}};

use super::IvyApplication;

impl IvyApplication {
    pub fn get_terminal_config(&self) -> (FontDescription, [RGBA; 2], [RGBA; 16], u32) {
        let config = self.imp().config.borrow();
        config.get_terminal_config()
    }

    pub fn handle_keyboard_event(&self, event: Event) -> Option<KeyboardAction> {
        let keybindings = self.imp().keybindings.borrow();
        check_keybinding_match(&keybindings, event)
    }

    pub fn get_keybindings(&self) -> Vec<Keybinding> {
        let keybindings = self.imp().keybindings.borrow();
        keybindings.clone()
    }

    pub fn update_keybinding(&self, old: &Keybinding, new_trigger: &Option<ShortcutTrigger>) {
        let mut keybindings = self.imp().keybindings.borrow_mut();
        for keybinding in keybindings.iter_mut() {
            // Update the Trigger for the correct Keybinding
            if keybinding.action == old.action {
                keybinding.trigger = new_trigger.clone();
                continue;
            }
            // If another Keybinding has the same Trigger as the new one, unassign it
            if keybinding.trigger.eq(new_trigger) {
                keybinding.trigger = None;
            }
        }
    }

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
