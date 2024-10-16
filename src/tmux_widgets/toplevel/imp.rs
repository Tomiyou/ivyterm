use std::cell::{Cell, RefCell};

use gtk4::Widget;
use libadwaita::{glib, subclass::prelude::*};

use crate::tmux_widgets::{container::TmuxContainer, terminal::TmuxTerminal, IvyTmuxWindow};

use super::layout::TopLevelLayout;

pub struct Zoomed {
    pub terminal: TmuxTerminal,
    pub root_container: TmuxContainer,
    pub terminal_container: TmuxContainer,
    pub previous_sibling: Option<Widget>,
    pub previous_bounds: (i32, i32, i32, i32),
}

// Object holding the state
#[derive(Default)]
pub struct TopLevelPriv {
    pub tab_id: Cell<u32>,
    pub window: RefCell<Option<IvyTmuxWindow>>,
    // TODO: Replace this with SortedVec
    pub terminals: RefCell<Vec<TmuxTerminal>>,
    pub zoomed: RefCell<Option<Zoomed>>,
    pub focused_terminal: Cell<u32>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TopLevelPriv {
    const NAME: &'static str = "ivytermTmuxTabPage";
    type Type = super::TmuxTopLevel;
    type ParentType = libadwaita::Bin;

    fn class_init(gtk_class: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        gtk_class.set_layout_manager_type::<TopLevelLayout>();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for TopLevelPriv {}

// Trait shared by all widgets
impl WidgetImpl for TopLevelPriv {}

// Trait shared by all Bins
impl BinImpl for TopLevelPriv {}

impl TopLevelPriv {
    pub fn init_values(&self, window: &IvyTmuxWindow, tab_id: u32) {
        self.window.replace(Some(window.clone()));
        self.tab_id.replace(tab_id);
    }
}
