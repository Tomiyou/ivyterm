use std::cell::RefCell;
use std::sync::atomic::AtomicU32;

use gtk4::CssProvider;
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, ApplicationWindow, TabView};

use crate::helpers::SortedVec;
use crate::normal_widgets::terminal::Terminal;
use crate::normal_widgets::toplevel::TopLevel;

// Object holding the state
#[derive(Default)]
pub struct IvyWindowPriv {
    pub tab_view: RefCell<Option<TabView>>,
    pub tabs: RefCell<Vec<TopLevel>>,
    pub terminals: RefCell<SortedVec<Terminal>>,
    pub css_provider: RefCell<CssProvider>,
    pub next_tab_id: RefCell<AtomicU32>,
    pub next_terminal_id: RefCell<AtomicU32>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyWindowPriv {
    const NAME: &'static str = "IvyApplicationWindow";
    type Type = super::IvyNormalWindow;
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
