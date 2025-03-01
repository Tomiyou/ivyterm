use std::cell::{Cell, RefCell};

use libadwaita::{glib, prelude::*, subclass::prelude::*};
use vte4::Terminal as Vte;

// Object holding the state
#[derive(Default)]
pub struct TerminalPriv {
    pub vte: RefCell<Option<Vte>>,
    pub id: Cell<u32>,
    initial_output: Cell<bool>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TerminalPriv {
    const NAME: &'static str = "ivytermTmuxTerminal";
    type Type = super::TmuxTerminal;
    type ParentType = libadwaita::Bin;
}

// Trait shared by all GObjects
impl ObjectImpl for TerminalPriv {
    fn dispose(&self) {
        self.vte.take();
    }
}

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
    pub fn init_values(&self, id: u32, terminal: &Vte) {
        self.id.replace(id);
        self.vte.borrow_mut().replace(terminal.clone());
    }

    pub fn is_synced(&self) -> bool {
        self.initial_output.get()
    }

    pub fn set_synced(&self) {
        self.initial_output.replace(true);
    }
}
