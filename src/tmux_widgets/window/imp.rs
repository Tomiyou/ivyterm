use std::cell::{Cell, RefCell};

use gtk4::CssProvider;
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, ApplicationWindow, TabView};

use crate::helpers::SortedVec;
use crate::tmux_api::TmuxAPI;
use crate::tmux_widgets::terminal::TmuxTerminal;
use crate::tmux_widgets::toplevel::TmuxTopLevel;

// Object holding the state
#[derive(Default)]
pub struct IvyWindowPriv {
    pub tmux: RefCell<Option<TmuxAPI>>,
    pub tab_view: RefCell<Option<TabView>>,
    pub tabs: RefCell<Vec<TmuxTopLevel>>,
    pub terminals: RefCell<SortedVec<TmuxTerminal>>,
    pub css_provider: RefCell<CssProvider>,
    pub char_size: Cell<(i32, i32)>,
    pub focused_tab: Cell<u32>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyWindowPriv {
    const NAME: &'static str = "ivytermTmuxWindow";
    type Type = super::IvyTmuxWindow;
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

impl IvyWindowPriv {
    pub fn initialize(&self, tab_view: &TabView, css_provider: &CssProvider) {
        let mut binding = self.tab_view.borrow_mut();
        binding.replace(tab_view.clone());

        self.css_provider.replace(css_provider.clone());
    }
}
