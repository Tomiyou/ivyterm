mod imp;

use glib::Object;
use gtk4::Widget;
use libadwaita::{glib, prelude::*};
use libadwaita::subclass::prelude::ObjectSubclassIsExt;

use super::{separator::Separator, Container};

glib::wrapper! {
    pub struct TmuxLayout(ObjectSubclass<imp::TmuxLayoutPriv>)
        @extends gtk4::LayoutManager;
}

impl TmuxLayout {
    pub fn new() -> Self {
        Object::builder().build()
    }

    fn get_terminal_count(&self) -> usize {
        self.imp().separators.borrow().len() + 1
    }

    pub fn add_separator(&self, container: &Container) -> Separator {
        todo!()
    }

    pub fn remove_separator(&self, removed: Option<Widget>) -> usize {
        todo!()
    }
}
