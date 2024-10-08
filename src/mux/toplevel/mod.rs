mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::Orientation;
use libadwaita::{glib, prelude::*, TabView};
use vte4::WidgetExt;

use crate::mux::{pane::new_paned, terminal::create_terminal};

use super::create_tab;

glib::wrapper! {
    pub struct TopLevel(ObjectSubclass<imp::TopLevel>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl TopLevel {
    pub fn new(tab_view: &TabView) -> Self {
        let top_level: TopLevel = Object::builder().build();

        top_level.imp().tab_view.borrow_mut().replace(tab_view.clone());

        let terminal = create_terminal(&top_level);

        top_level.set_vexpand(true);
        top_level.set_hexpand(true);
        top_level.set_focusable(true);
        top_level.set_child(Some(&terminal));

        top_level
    }

    pub fn create_tab(&self) {
        let binding = self.imp().tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();
        create_tab(tab_view);
    }

    pub fn close_tab(&self) {
        let binding = self.imp().tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();
        let page = tab_view.page(self);
        tab_view.close_page(&page);
    }

    pub fn split(&self, orientation: Orientation) {
        let old_terminal = self.child().unwrap();
        let new_terminal = create_terminal(&self);

        self.set_child(None::<&Self>);
        let new_paned = new_paned(orientation, old_terminal, new_terminal);
        self.set_child(Some(&new_paned));

        println!("New PANE {:?}", new_paned.as_ptr())
    }
}
