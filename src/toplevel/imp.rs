use std::cell::RefCell;

use gtk4::{Box as Container, Widget};
use libadwaita::{glib, subclass::prelude::*, TabView};

use crate::{pane::Pane, window::IvyWindow};

pub struct Zoomed {
    pub terminal: Pane,
    pub root_container: Container,
    pub terminal_container: Container,
    pub previous_sibling: Option<Widget>,
    pub previous_bounds: (i32, i32, i32, i32),
}

// Object holding the state
#[derive(Default)]
pub struct TopLevelPriv {
    pub window: RefCell<Option<IvyWindow>>,
    pub tab_view: RefCell<Option<TabView>>,
    pub terminals: RefCell<Vec<Pane>>,
    pub zoomed: RefCell<Option<Zoomed>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TopLevelPriv {
    const NAME: &'static str = "TopLevelTerminalContainer";
    type Type = super::TopLevel;
    type ParentType = libadwaita::Bin;
}

// Trait shared by all GObjects
impl ObjectImpl for TopLevelPriv {}

// Trait shared by all widgets
impl WidgetImpl for TopLevelPriv {}

// Trait shared by all Bins
impl BinImpl for TopLevelPriv {}

impl TopLevelPriv {
    pub fn init_values(&self, tab_view: &TabView, window: &IvyWindow) {
        self.window.replace(Some(window.clone()));
        self.tab_view.replace(Some(tab_view.clone()));
    }
}
