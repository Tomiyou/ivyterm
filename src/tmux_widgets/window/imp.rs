use std::cell::{Cell, RefCell};

use glib::Propagation;
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, ApplicationWindow, TabView};

use crate::helpers::SortedVec;
use crate::tmux_api::TmuxAPI;
use crate::tmux_widgets::terminal::TmuxTerminal;
use crate::tmux_widgets::toplevel::TmuxTopLevel;

use super::tmux::TmuxInitState;

// Object holding the state
#[derive(Default)]
pub struct IvyWindowPriv {
    pub tmux: RefCell<Option<TmuxAPI>>,
    pub tab_view: RefCell<Option<TabView>>,
    // TODO: Use SortedVec
    pub tabs: RefCell<Vec<TmuxTopLevel>>,
    pub terminals: RefCell<SortedVec<TmuxTerminal>>,
    pub char_size: Cell<(i32, i32)>,
    pub focused_tab: Cell<u32>,
    pub session: Cell<Option<(u32, String)>>,
    pub init_layout_finished: Cell<TmuxInitState>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyWindowPriv {
    const NAME: &'static str = "ivytermTmuxWindow";
    type Type = super::IvyTmuxWindow;
    type ParentType = ApplicationWindow;
}

// Trait shared by all GObjects
impl ObjectImpl for IvyWindowPriv {
    fn dispose(&self) {
        self.tmux.take();
        self.tabs.borrow_mut().clear();
        self.terminals.borrow_mut().clear();

        // Close all remaining pages
        self.tab_view.take();
    }
}

// Trait shared by all widgets
impl WidgetImpl for IvyWindowPriv {}

// Trait shared by all windows
impl WindowImpl for IvyWindowPriv {
    fn close_request(&self) -> Propagation {
        // Clear Tabs and Terminals
        self.tmux.take();
        self.terminals.borrow_mut().clear();
        self.tabs.borrow_mut().clear();

        // Close all TabView pages
        if let Some(tab_view) = self.tab_view.take() {
            if tab_view.n_pages() > 0 {
                let first_page = tab_view.nth_page(0);
                tab_view.close_other_pages(&first_page);
                tab_view.close_page(&first_page);
            }
        }

        Propagation::Proceed
    }
}

// Trait shared by all application windows
impl ApplicationWindowImpl for IvyWindowPriv {}

// Trait shared by all Adwaita application windows
impl AdwApplicationWindowImpl for IvyWindowPriv {}

impl IvyWindowPriv {
    pub fn initialize(&self, tab_view: &TabView) {
        let mut binding = self.tab_view.borrow_mut();
        binding.replace(tab_view.clone());
    }
}
