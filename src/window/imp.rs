use std::cell::RefCell;
use std::collections::HashMap;

use libadwaita::subclass::prelude::*;
use libadwaita::{glib, ApplicationWindow, TabView};

use crate::pane::Pane;
use crate::tmux::Tmux;
use crate::toplevel::TopLevel;

// Object holding the state
#[derive(Default)]
pub struct IvyWindowPriv {
    pub tmux: RefCell<Option<Tmux>>,
    pub tab_view: RefCell<Option<TabView>>,
    pub tabs: RefCell<Vec<TopLevel>>,
    pub terminals: RefCell<HashMap<u32, Pane>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyWindowPriv {
    const NAME: &'static str = "IvyApplicationWindow";
    type Type = super::IvyWindow;
    type ParentType = ApplicationWindow;
}

// Trait shared by all GObjects
impl ObjectImpl for IvyWindowPriv {}

// Trait shared by all widgets
impl WidgetImpl for IvyWindowPriv {}

// Trait shared by all windows
impl WindowImpl for IvyWindowPriv {}

// Trait shared by all application windows
impl ApplicationWindowImpl for IvyWindowPriv {}

// Trait shared by all Adwaita application windows
impl AdwApplicationWindowImpl for IvyWindowPriv {}
