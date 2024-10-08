use std::cell::{Cell, RefCell};

use glib::subclass::{object::ObjectImpl, types::ObjectSubclass};
use gtk4::{subclass::widget::WidgetImpl, Orientation};
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, prelude::*, Bin};

// Object holding the state
#[derive(Debug, glib::Properties)]
#[properties(wrapper_type = super::TmuxSeparator)]
pub struct SeparatorPriv {
    // Left/top position of handle
    pub position: Cell<i32>,
    #[property(get, set=Self::set_orientation, builder(gtk4::Orientation::Horizontal))]
    orientation: RefCell<gtk4::Orientation>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SeparatorPriv {
    const NAME: &'static str = "IvyTerminalSeparator";
    type Type = super::TmuxSeparator;
    type ParentType = Bin;
    type Interfaces = (gtk4::Orientable,);

    fn new() -> Self {
        // Here we set the default orientation.
        Self {
            position: Cell::new(0),
            orientation: RefCell::new(Orientation::Horizontal),
        }
    }
}

// Trait shared by all GObjects
#[glib::derived_properties]
impl ObjectImpl for SeparatorPriv {}

// Trait shared by all widgets
impl WidgetImpl for SeparatorPriv {}

// Trait shared by all buttons
impl BinImpl for SeparatorPriv {}

impl OrientableImpl for SeparatorPriv {}

impl SeparatorPriv {
    pub fn set_orientation(&self, orientation: Orientation) {
        self.orientation.replace(orientation);
    }
}
