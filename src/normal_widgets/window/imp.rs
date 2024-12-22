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
        self.tab_view.take();
    }
}

// Trait shared by all widgets
impl WidgetImpl for IvyWindowPriv {}

// Trait shared by all windows
impl WindowImpl for IvyWindowPriv {
    fn close_request(&self) -> Propagation {
        let mut terminals = self.terminals.borrow_mut();

        // If there are 2 or more Terminals remaining in our Window, user needs to confirm
        // that he really wants to close the window
        if terminals.len() > 1 && !self.close_allowed.get() {
            let window = self.obj();
            let allow_close = glib::closure_local!(
                #[weak]
                window,
                move || {
                    window.imp().close_allowed.replace(true);
                }
            );
            // Spawn exit confirm modal
            spawn_exit_modal(window.upcast_ref(), allow_close);
            return Propagation::Stop;
        }

        // TODO: This feels hacky..
        self.obj().set_content(None::<&gtk4::Widget>);
        // Clear Tabs and Terminals
        terminals.clear();
        drop(terminals);
        self.tabs.borrow_mut().clear();

        // Close all TabView pages
        if let Some(tab_view) = self.tab_view.take() {
            if tab_view.n_pages() > 0 {
                let first_page = tab_view.nth_page(0);
                tab_view.close_other_pages(&first_page);
                tab_view.close_page(&first_page);

                // This is a hacky fix of what appears to be a libadwaita issue.
                // The issue is reproducible in 1.5.0 and resolved in 1.6.0. Not
                // sure if 1.5.x versions have been fixed.
                if libadwaita::major_version() < 2 && libadwaita::minor_version() < 6 {
                    first_page.child().unparent();
                }
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
