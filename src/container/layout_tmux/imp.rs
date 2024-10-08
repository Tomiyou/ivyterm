use std::cell::RefCell;

use gtk4::{Allocation, LayoutManager, Orientation};
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, Bin};
use vte4::{Cast, WidgetExt};

use crate::container::separator::Separator;
use crate::container::Container;

// Object holding the state
#[derive(Default)]
pub struct TmuxLayoutPriv {
    pub separators: RefCell<Vec<Separator>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TmuxLayoutPriv {
    const NAME: &'static str = "IvyTerminalTmuxLayout";
    type Type = super::TmuxLayout;
    type ParentType = LayoutManager;
}

// Trait shared by all GObjects
impl ObjectImpl for TmuxLayoutPriv {}

impl LayoutManagerImpl for TmuxLayoutPriv {
    fn measure(
        &self,
        widget: &gtk4::Widget,
        orientation: gtk4::Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        todo!()
    }

    fn allocate(&self, widget: &gtk4::Widget, width: i32, height: i32, _baseline: i32) {
        todo!()
    }
}
