mod imp;
mod layout;
mod separator;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{Orientation, Widget};
use libadwaita::{glib, prelude::*};

pub use layout::ContainerLayout;

use super::window::IvyNormalWindow;

glib::wrapper! {
    pub struct Container(ObjectSubclass<imp::ContainerPriv>)
        @extends Widget,
        @implements gtk4::Orientable;
}

impl Container {
    pub fn new(orientation: Orientation) -> Self {
        let container: Self = Object::builder().build();
        container.set_orientation(orientation);
        container.set_vexpand(true);
        container.set_hexpand(true);

        let imp = container.imp();

        let layout: ContainerLayout = container.layout_manager().unwrap().downcast().unwrap();
        imp.layout.replace(Some(layout));

        container
    }

    pub fn append(&self, child: &impl IsA<Widget>) {
        let imp = self.imp();
        if let Some(last_child) = self.last_child() {
            let binding = imp.layout.borrow();
            let layout = binding.as_ref().unwrap();
            let new_separator = layout.add_separator(self);

            new_separator.insert_after(self, Some(&last_child));
            child.insert_after(self, Some(&new_separator));
        } else {
            child.insert_after(self, None::<&Widget>);
        }
    }

    pub fn prepend(&self, child: &impl IsA<Widget>, sibling: &Option<impl IsA<Widget>>) {
        // TODO: Prepend on sibling None means append() last...
        if let Some(sibling) = sibling {
            let imp = self.imp();
            let binding = imp.layout.borrow();
            let layout = binding.as_ref().unwrap();
            let new_separator = layout.add_separator(self);

            child.insert_before(self, Some(sibling));
            new_separator.insert_after(self, Some(child));
        } else {
            self.append(child);
        }
    }

    pub fn replace(&self, old: &impl IsA<Widget>, new: &impl IsA<Widget>) {
        new.insert_after(self, Some(old));
        old.unparent();
    }

    pub fn remove(&self, child: &impl IsA<Widget>) -> usize {
        let separator = child.next_sibling();
        let binding = self.imp().layout.borrow();
        let layout = binding.as_ref().unwrap();
        let len = layout.remove_separator(separator);

        // Now remove the child
        child.unparent();

        len
    }
}
