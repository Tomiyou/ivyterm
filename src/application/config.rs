use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::{gdk::Event, ShortcutTrigger};

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

    pub fn update_keybinding(&self, old: &Keybinding, new_trigger: &Option<ShortcutTrigger>) {
        let mut config = self.imp().config.borrow_mut();
        let mut keybindings = self.imp().keybindings.borrow_mut();
        for keybinding in keybindings.iter_mut() {
            // Update the Trigger for the correct Keybinding
            if keybinding.action == old.action {
                keybinding.trigger = new_trigger.clone();
                config.keybindings.update_one(keybinding);
                continue;
            }
            // If another Keybinding has the same Trigger as the new one, unassign it
            if keybinding.trigger.eq(new_trigger) {
                keybinding.trigger = None;
            }
        }

        // Write new configuration to file
        config.write_config_to_file();
    }

    pub fn update_config(&self, new: GlobalConfig) {
        // Write config to file
        new.write_config_to_file();
        self.imp().config.replace(new);

        // Now reload the widgets
        self.reload_css_colors();
        self.refresh_terminals();
    }
}
