use std::cell::RefCell;

use libadwaita::{glib, subclass::prelude::*, TabView};

// Object holding the state
#[derive(Default)]
pub struct TopLevel {
    pub tab_view: RefCell<Option<TabView>>,
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
