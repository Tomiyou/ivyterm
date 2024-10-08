use std::cell::RefCell;

use gtk4::Paned;
use libadwaita::{glib, subclass::prelude::*, TabView};

use crate::terminal::IvyTerminal;

pub struct Zoomed {
    pub terminal: IvyTerminal,
    pub root_paned: Paned,
    pub terminal_paned: Paned,
    pub is_start_child: bool,
}

// Object holding the state
#[derive(Default)]
pub struct TopLevel {
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
