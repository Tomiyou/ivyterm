use std::cell::{Cell, RefCell};
use std::sync::atomic::AtomicU32;

use glib::Propagation;
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, prelude::*, ApplicationWindow, TabView};

use crate::helpers::SortedVec;
use crate::modals::spawn_exit_modal;
use crate::normal_widgets::terminal::Terminal;
use crate::normal_widgets::toplevel::TopLevel;

// Object holding the state
#[derive(Default)]
pub struct IvyWindowPriv {
    pub tab_view: RefCell<Option<TabView>>,
    pub tabs: RefCell<Vec<TopLevel>>,
    pub terminals: RefCell<SortedVec<Terminal>>,
    pub next_tab_id: AtomicU32,
    pub next_terminal_id: AtomicU32,
    pub close_allowed: Cell<bool>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyWindowPriv {
    const NAME: &'static str = "ivytermWindow";
    type Type = super::IvyNormalWindow;
    type ParentType = ApplicationWindow;
}

// Trait shared by all GObjects
impl ObjectImpl for IvyWindowPriv {
    fn dispose(&self) {
        // Remove all remaining Tabs
        self.tabs.borrow_mut().clear();
        self.terminals.borrow_mut().clear();

        // Close all remaining pages
        if let Some(tab_view) = self.tab_view.take() {
            if tab_view.n_pages() > 0 {
                let first_page = tab_view.nth_page(0);
                tab_view.close_pages_after(&first_page);
                tab_view.close_page(&first_page);
            }
        }
    }
}

// Trait shared by all widgets
impl WidgetImpl for IvyWindowPriv {}

// Trait shared by all windows
impl WindowImpl for IvyWindowPriv {
    fn close_request(&self) -> Propagation {
        let terminal_count = self.terminals.borrow().len();

        // If there are no Terminals open, we can close immediately
        if terminal_count < 1 {
            return Propagation::Proceed;
        }

        // If user confirmed close, we can start closing tabs, then wait for
        // dispose to call tab_closed() when everything is actually released
        if self.close_allowed.get() || terminal_count < 2 {
            self.tabs.borrow_mut().clear();

            let tab_view = self.tab_view.take().unwrap();
            if tab_view.n_pages() > 0 {
                let first_page = tab_view.nth_page(0);
                tab_view.close_other_pages(&first_page);
                tab_view.close_page(&first_page);
            }
            return Propagation::Stop;
        }

        // If there are more than 2 terminals left, ask the user if he really wants
        // to close the window first
        let window = self.obj();
        let allow_close = glib::closure_local!(
            #[weak]
            window,
            move || {
                window.imp().close_allowed.replace(true);
            }
        );
        spawn_exit_modal(window.upcast_ref(), allow_close);

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
