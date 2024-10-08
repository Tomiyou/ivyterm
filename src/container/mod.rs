mod imp;
mod layout;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{Orientation, Widget};
use libadwaita::{glib, prelude::*, Bin};

glib::wrapper! {
    pub struct Container(ObjectSubclass<imp::ContainerPriv>)
        @extends Widget,
        @implements gtk4::Orientable;
}

impl Container {
    pub fn new(
        orientation: Orientation,
        start_child: impl IsA<Widget>,
        end_child: impl IsA<Widget>,
    ) -> Self {
        let container: Self = Object::builder().build();
        container.set_orientation(orientation);
        container.set_vexpand(true);
        container.set_hexpand(true);

        // Add the given children as panes
        container.set_start_child(Some(&start_child));
        container.set_end_child(Some(&end_child));

        container
    }

    pub fn set_start_child(&self, new_child: Option<&impl IsA<Widget>>) {
        let imp = self.imp();
        let mut start_child = imp.start_child.borrow_mut();

        // TODO: Check if old_child and new_child are the same
        if let Some(old_child) = start_child.take() {
            old_child.unparent();
        }

        if let Some(new_child) = new_child {
            start_child.replace(new_child.clone().into());

            let separator = imp.separator.borrow();
            new_child.insert_before(self, Some(separator.as_ref().unwrap()));
        }
    }

    pub fn set_end_child(&self, new_child: Option<&impl IsA<Widget>>) {
        let imp = self.imp();
        let mut end_child = imp.end_child.borrow_mut();

        // TODO: Check if old_child and new_child are the same
        if let Some(old_child) = end_child.take() {
            old_child.unparent();
        }

        if let Some(new_child) = new_child {
            end_child.replace(new_child.clone().into());

            let separator = imp.separator.borrow();
            new_child.insert_after(self, Some(separator.as_ref().unwrap()));
        }
    }

    pub fn start_child(&self) -> Option<Widget> {
        self.imp().start_child.borrow().clone()
    }

    pub fn end_child(&self) -> Option<Widget> {
        self.imp().end_child.borrow().clone()
    }

    pub fn get_separator(&self) -> Bin {
        let imp = self.imp();
        let bin = imp.separator.borrow().as_ref().unwrap().clone();
        bin
    }
}
