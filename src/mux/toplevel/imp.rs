use std::cell::RefCell;

use libadwaita::subclass::prelude::*;
use libadwaita::{glib, TabPage, TabView};

// Object holding the state
#[derive(Default)]
pub struct TopLevel {
    pub tab_view: RefCell<Option<(TabView, TabPage)>>,
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
impl WidgetImpl for TopLevel {
    fn unrealize(&self) {
        self.parent_unrealize();

        let binding = self.tab_view.borrow();
        let (view, page) = binding.as_ref().unwrap();
        view.close_page(&page);
    }
}

// Trait shared by all Bins
impl BinImpl for TopLevel {}
