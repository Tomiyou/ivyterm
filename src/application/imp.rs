use std::cell::RefCell;

use gtk4::CssProvider;
use libadwaita::glib;
use libadwaita::subclass::prelude::*;

use crate::config::GlobalConfig;
use crate::keyboard::Keybinding;

// Object holding the state
#[derive(Default)]
pub struct IvyApplicationPriv {
    pub css_provider: RefCell<Option<CssProvider>>,
    pub config: RefCell<GlobalConfig>,
    pub keybindings: RefCell<Vec<Keybinding>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyApplicationPriv {
    const NAME: &'static str = "ivytermApplication";
    type Type = super::IvyApplication;
    type ParentType = libadwaita::Application;
}

impl ObjectImpl for IvyApplicationPriv {
    fn dispose(&self) {
        self.css_provider.take();
        self.keybindings.borrow_mut().clear();
    }
}
impl ApplicationImpl for IvyApplicationPriv {}
impl GtkApplicationImpl for IvyApplicationPriv {}
impl AdwApplicationImpl for IvyApplicationPriv {}
