use std::cell::{Cell, RefCell};

use gtk4::{Box, Widget};
use libadwaita::{glib, Bin};
use libadwaita::subclass::prelude::*;

// Object holding the state
#[derive(Default)]
pub struct IvyPanedPriv {
    pub separator: RefCell<Option<Bin>>,
    pub separator_visible: Cell<bool>,
    pub start_child: RefCell<Option<Widget>>,
    pub end_child: RefCell<Option<Widget>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyPanedPriv {
    const NAME: &'static str = "IvyTerminalCustomPaned";
    type Type = super::IvyPaned;
    type ParentType = Box;
}

// Trait shared by all GObjects
impl ObjectImpl for IvyPanedPriv {}

// Trait shared by all widgets
impl WidgetImpl for IvyPanedPriv {}

// Trait shared by all buttons
impl BoxImpl for IvyPanedPriv {}
