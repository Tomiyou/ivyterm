use std::cell::RefCell;

use libadwaita::{glib, subclass::prelude::*};
use vte4::{Terminal, WidgetExt};

use crate::window::IvyWindow;

// Object holding the state
#[derive(Default)]
pub struct PanePriv {
    terminal: RefCell<Option<Terminal>>,
    window: RefCell<Option<IvyWindow>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for PanePriv {
    const NAME: &'static str = "IvyTerminalCustomTerminal";
    type Type = super::Pane;
    type ParentType = libadwaita::Bin;
}

// Trait shared by all GObjects
impl ObjectImpl for PanePriv {}

// Trait shared by all widgets
impl WidgetImpl for PanePriv {
    fn grab_focus(&self) -> bool {
        self.parent_grab_focus();

        self.terminal.borrow().as_ref().unwrap().grab_focus()
    }
}

// Trait shared by all buttons
impl BinImpl for PanePriv {}

impl PanePriv {
    pub fn init_values(&self, terminal: &Terminal, window: &IvyWindow) {
        self.terminal.borrow_mut().replace(terminal.clone());
        self.window.borrow_mut().replace(window.clone());
    }
}
