use std::cell::{Cell, RefCell};

use libadwaita::{glib, subclass::prelude::*};
use vte4::{Terminal as Vte, WidgetExt};

use crate::normal_widgets::window::IvyNormalWindow;

// Object holding the state
#[derive(Default)]
pub struct TerminalPriv {
    pub vte: RefCell<Option<Vte>>,
    window: RefCell<Option<IvyNormalWindow>>,
    pub id: Cell<u32>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TerminalPriv {
    const NAME: &'static str = "IvyTerminalCustomTerminal";
    type Type = super::Terminal;
    type ParentType = libadwaita::Bin;
}

// Trait shared by all GObjects
impl ObjectImpl for TerminalPriv {}

// Trait shared by all widgets
impl WidgetImpl for TerminalPriv {
    fn grab_focus(&self) -> bool {
        self.parent_grab_focus();

        self.vte.borrow().as_ref().unwrap().grab_focus()
    }
}

// Trait shared by all buttons
impl BinImpl for TerminalPriv {}

impl TerminalPriv {
    pub fn init_values(&self, id: u32, terminal: &Vte, window: &IvyNormalWindow) {
        self.id.replace(id);
        self.vte.borrow_mut().replace(terminal.clone());
        self.window.borrow_mut().replace(window.clone());
    }
}
