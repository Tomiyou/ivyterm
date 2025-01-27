use std::cell::{Cell, RefCell};

use gtk4::{EventControllerKey, Label};
use libadwaita::glib;
use libadwaita::{prelude::*, subclass::prelude::*};

use crate::helpers::borrow_clone;
use crate::keyboard::Keybinding;

use super::set_text_from_trigger;

// Object holding the state
#[derive(Default)]
pub struct KeybindingPage {
    pub keyboard_ctrl: RefCell<Option<EventControllerKey>>,
    pub keybindings: RefCell<Vec<(Keybinding, Label)>>,
    pub listening: Cell<Option<usize>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for KeybindingPage {
    const NAME: &'static str = "MyGtkAppCustomButton";
    type Type = super::KeybindingPage;
    type ParentType = libadwaita::PreferencesPage;
}

// Trait shared by all GObjects
impl ObjectImpl for KeybindingPage {
    fn dispose(&self) {
        // Remove keyboard controller
        self.keyboard_ctrl.take();

        // Remove all keybindings
        let mut keybindings = self.keybindings.borrow_mut();
        keybindings.clear();
    }
}

// Trait shared by all widgets
impl WidgetImpl for KeybindingPage {}

impl PreferencesPageImpl for KeybindingPage {}

impl KeybindingPage {
    pub fn enable_keyboard(&self, enable: bool) {
        let keyboard_ctrl = borrow_clone(&self.keyboard_ctrl);
        let obj = self.obj();

        if enable {
            obj.add_controller(keyboard_ctrl.clone());
        } else {
            obj.remove_controller(&keyboard_ctrl);
        }
    }

    pub fn update_label_text(&self, idx: usize, begin_listen: bool) {
        let keybindings = self.keybindings.borrow();
        let (keybinding, label) = keybindings.get(idx).unwrap();

        if begin_listen {
            label.set_label("Enter new keybinding...");
        } else {
            set_text_from_trigger(label, &keybinding.trigger);
        }
    }
}
