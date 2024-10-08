use std::cell::RefCell;

use libadwaita::{glib, subclass::prelude::*};
use vte4::{Terminal, WidgetExt};

// Object holding the state
#[derive(Default)]
pub struct IvyTerminalPriv {
    terminal: RefCell<Option<Terminal>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyTerminalPriv {
    const NAME: &'static str = "IvyTerminalCustomTerminal";
    type Type = super::IvyTerminal;
    type ParentType = libadwaita::Bin;
}

// Trait shared by all GObjects
impl ObjectImpl for IvyTerminalPriv {}

// Trait shared by all widgets
impl WidgetImpl for IvyTerminalPriv {
    fn grab_focus(&self) -> bool {
        self.parent_grab_focus();

        self.terminal.borrow().as_ref().unwrap().grab_focus()
    }
}

// Trait shared by all buttons
impl BinImpl for IvyTerminalPriv {}

impl IvyTerminalPriv {
    pub fn set_terminal(&self, terminal: &Terminal) {
        self.terminal.borrow_mut().replace(terminal.clone());
    }
}
