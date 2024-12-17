use std::cell::{Cell, RefCell};

use gtk4::Widget;
use libadwaita::{glib, subclass::prelude::*, TabView};

use crate::tmux_widgets::{container::TmuxContainer, terminal::TmuxTerminal, IvyTmuxWindow};

use super::layout::TopLevelLayout;

pub struct Zoomed {
    pub term_id: u32,
    pub terminal: TmuxTerminal,
    pub root_container: TmuxContainer,
    pub terminal_container: TmuxContainer,
    pub previous_sibling: Option<Widget>,
}

// Object holding the state
#[derive(Default)]
pub struct TopLevelPriv {
    pub tab_id: Cell<u32>,
    pub window: RefCell<Option<IvyTmuxWindow>>,
    pub tab_view: RefCell<Option<TabView>>,
    // TODO: Replace this with SortedVec
    pub terminals: RefCell<Vec<TmuxTerminal>>,
    pub zoomed: RefCell<Option<Zoomed>>,
    pub focused_terminal: Cell<u32>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TopLevelPriv {
    const NAME: &'static str = "ivytermTmuxTabPage";
    type Type = super::TmuxTopLevel;
    type ParentType = libadwaita::Bin;

    fn class_init(gtk_class: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        gtk_class.set_layout_manager_type::<TopLevelLayout>();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for TopLevelPriv {
    fn dispose(&self) {
        self.tab_view.take();

        let mut terminals = self.terminals.borrow_mut();
        let mut term_ids = Vec::new();
        for terminal in terminals.iter() {
            term_ids.push(terminal.id());
        }
        terminals.clear();

        if let Some(window) = self.window.take() {
            window.tab_closed(self.tab_id.get(), term_ids);
        }

        self.zoomed.take();
    }
}

// Trait shared by all widgets
impl WidgetImpl for TopLevelPriv {}

// Trait shared by all Bins
impl BinImpl for TopLevelPriv {}

impl TopLevelPriv {
    pub fn init_values(&self, tab_view: &TabView, window: &IvyTmuxWindow, tab_id: u32) {
        self.window.replace(Some(window.clone()));
        self.tab_view.replace(Some(tab_view.clone()));
        self.tab_id.replace(tab_id);
    }
}
