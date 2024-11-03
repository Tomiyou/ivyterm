use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::gdk::Event;

use crate::{
    config::{GlobalConfig, TerminalConfig},
    keyboard::{check_keybinding_match, Keybinding, KeyboardAction},
};

use super::IvyApplication;

impl IvyApplication {
    pub fn get_terminal_config(&self) -> TerminalConfig {
        let config = self.imp().config.borrow();
        config.terminal.clone()
    }

    pub fn handle_keyboard_event(&self, event: Event) -> Option<KeyboardAction> {
        let keybindings = self.imp().keybindings.borrow();
        check_keybinding_match(&keybindings, event)
    }

    pub fn get_keybindings(&self) -> Vec<Keybinding> {
        let keybindings = self.imp().keybindings.borrow();
        keybindings.clone()
    }

    pub fn update_config(&self, new: GlobalConfig, keybindings: Vec<Keybinding>) {
        // Write config to file
        new.write_config_to_file();

        let imp = self.imp();
        imp.keybindings.replace(keybindings);
        imp.config.replace(new);

        // Now reload the widgets
        self.reload_css();
        self.refresh_terminals();
    }
}
