use std::{cell::{Cell, RefCell}, rc::Rc};

use libadwaita::{glib, subclass::prelude::*};
use vte4::{Terminal, WidgetExt};

use crate::global_state::WindowState;

// Object holding the state
#[derive(Default)]
pub struct PanePriv {
    terminal: RefCell<Option<Terminal>>,
    window_state: RefCell<Option<Rc<WindowState>>>,
    is_tmux: Cell<bool>,
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
    pub fn init_values(&self, terminal: &Terminal, window_state: Rc<WindowState>) {
        self.is_tmux.replace(window_state.tmux);
        self.terminal.borrow_mut().replace(terminal.clone());
        self.window_state.replace(Some(window_state));
    }
}
