use std::cell::{Cell, RefCell};

use gtk4::Widget;
use libadwaita::{glib, subclass::prelude::*, TabView};

use crate::{
    helpers::WithId,
    normal_widgets::{container::Container, terminal::Terminal, window::IvyNormalWindow},
};

use super::layout::TopLevelLayout;

pub struct Zoomed {
    pub terminal: Terminal,
    pub root_container: Container,
    pub terminal_container: Container,
    pub previous_sibling: Option<Widget>,
    pub previous_bounds: (i32, i32, i32, i32),
}

// Object holding the state
#[derive(Default)]
pub struct TopLevelPriv {
    pub window: RefCell<Option<IvyNormalWindow>>,
    pub tab_view: RefCell<Option<TabView>>,
    pub terminals: RefCell<Vec<Terminal>>,
    pub lru_terminals: RefCell<Vec<WithId<Terminal>>>,
    pub zoomed: RefCell<Option<Zoomed>>,
    pub tab_id: Cell<u32>,
    pub name: RefCell<Option<String>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TopLevelPriv {
    const NAME: &'static str = "ivytermTabPage";
    type Type = super::TopLevel;
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

        let mut lru_terminals = self.lru_terminals.borrow_mut();
        lru_terminals.clear();

        self.zoomed.take();
        self.name.take();
    }
}

// Trait shared by all widgets
impl WidgetImpl for TopLevelPriv {}

// Trait shared by all Bins
impl BinImpl for TopLevelPriv {}

impl TopLevelPriv {
    pub fn init_values(&self, tab_view: &TabView, window: &IvyNormalWindow, tab_id: u32) {
        self.window.replace(Some(window.clone()));
        self.tab_view.replace(Some(tab_view.clone()));
        self.tab_id.replace(tab_id);
    }
}
