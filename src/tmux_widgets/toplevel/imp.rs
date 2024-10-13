use std::cell::{Cell, RefCell};

use gtk4::Widget;
use libadwaita::{glib, subclass::prelude::*, TabView};

use crate::{
    helpers::WithId,
    tmux_widgets::{container::TmuxContainer, terminal::TmuxTerminal, IvyTmuxWindow},
};

use super::layout::TopLevelLayout;

pub struct Zoomed {
    pub terminal: TmuxTerminal,
    pub root_container: TmuxContainer,
    pub terminal_container: TmuxContainer,
    pub previous_sibling: Option<Widget>,
    pub previous_bounds: (i32, i32, i32, i32),
}

// Object holding the state
#[derive(Default)]
pub struct TopLevelPriv {
    pub window: RefCell<Option<IvyTmuxWindow>>,
    pub tab_view: RefCell<Option<TabView>>,
    // TODO: Replace this with SortedVec
    pub terminals: RefCell<Vec<TmuxTerminal>>,
    pub lru_terminals: RefCell<Vec<WithId<TmuxTerminal>>>,
    pub zoomed: RefCell<Option<Zoomed>>,
    pub tab_id: Cell<u32>,
    pub focused_terminal: Cell<u32>,
    pub initialized: Cell<bool>,
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
impl ObjectImpl for TopLevelPriv {}

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
