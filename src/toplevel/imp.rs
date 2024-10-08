use std::cell::RefCell;

use libadwaita::{glib, subclass::prelude::*, TabView};

use crate::{global_state::WindowState, paned::IvyPaned, terminal::IvyTerminal};

pub struct Zoomed {
    pub terminal: IvyTerminal,
    pub root_paned: IvyPaned,
    pub terminal_paned: IvyPaned,
    pub is_start_child: bool,
    pub previous_bounds: (i32, i32, i32, i32),
}

// Object holding the state
#[derive(Default)]
pub struct TopLevel {
    pub window_state: RefCell<Option<WindowState>>,
    pub tab_view: RefCell<Option<TabView>>,
    pub terminals: RefCell<Vec<IvyTerminal>>,
    pub zoomed: RefCell<Option<Zoomed>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TopLevel {
    const NAME: &'static str = "TopLevelTerminalContainer";
    type Type = super::TopLevel;
    type ParentType = libadwaita::Bin;
}

// Trait shared by all GObjects
impl ObjectImpl for TopLevel {}

// Trait shared by all widgets
impl WidgetImpl for TopLevel {}

// Trait shared by all Bins
impl BinImpl for TopLevel {}
