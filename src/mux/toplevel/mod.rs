mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Propagation};
use gtk4::{EventControllerKey, Orientation};
use libadwaita::{glib, prelude::*, TabPage, TabView};
use vte4::WidgetExt;

use crate::{
    keyboard::matches_keybinding,
    mux::{pane::new_paned, terminal::create_terminal},
};

glib::wrapper! {
    pub struct TopLevel(ObjectSubclass<imp::TopLevel>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl TopLevel {
    pub fn new() -> Self {
        let terminal = create_terminal();

        let top_level: TopLevel = Object::builder().build();

        top_level.set_vexpand(true);
        top_level.set_hexpand(true);
        top_level.set_focusable(true);
        top_level.set_child(Some(&terminal));

        top_level
    }

    pub fn close_tab(&self) {
        self.unrealize();
    }

    pub fn split(&self, orientation: Orientation) {
        let old_terminal = self.child().unwrap();

        let new_terminal = create_terminal();
        let none: Option<&Self> = None;

        self.set_child(none);
        let new_paned = new_paned(orientation, old_terminal, new_terminal);
        self.set_child(Some(&new_paned));

        println!("New PANE {:?}", new_paned.as_ptr())
    }

    pub fn set_tab_view(&self, view: &TabView, page: &TabPage) {
        let imp = self.imp();
        imp.tab_view
            .borrow_mut()
            .replace((view.clone(), page.clone()));
    }
}
