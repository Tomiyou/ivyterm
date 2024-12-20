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
        if let Some(tab_view) = self.tab_view.take() {
            if tab_view.n_pages() > 0 {
                let first_page = tab_view.nth_page(0);
                tab_view.close_other_pages(&first_page);
                tab_view.close_page(&first_page);
            }
        }
    }
}

// Trait shared by all widgets
impl WidgetImpl for IvyWindowPriv {
    fn unrealize(&self) {
        self.parent_unrealize();

        // TODO: GTK code currently does NOT clean up closing window directly ...
        self.tmux.take();
        self.tab_view.take();
        self.terminals.borrow_mut().clear();
        self.tabs.borrow_mut().clear();
    }
}

// Trait shared by all windows
impl WindowImpl for IvyWindowPriv {
    fn close_request(&self) -> Propagation {
        let terminal_count = self.terminals.borrow().len();

        // If there are no Terminals open, we can close immediately
        if terminal_count < 1 {
            return Propagation::Proceed;
        }

        // Start closing tabs, then wait for dispose to call tab_closed()
        // when everything is actually released
        self.tabs.borrow_mut().clear();

        let tab_view = self.tab_view.take().unwrap();
        if tab_view.n_pages() > 0 {
            let first_page = tab_view.nth_page(0);
            tab_view.close_other_pages(&first_page);
            tab_view.close_page(&first_page);
        }

        // If all children were disposed while we were running, we can exit early
        let terminal_count = self.terminals.borrow().len();
        if terminal_count < 1 {
            return Propagation::Proceed;
        }

        Propagation::Stop
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
