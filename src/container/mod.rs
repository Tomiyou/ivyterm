mod imp;
mod layout;
mod separator;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{Orientation, Widget};
use libadwaita::{glib, prelude::*};

glib::wrapper! {
    pub struct Container(ObjectSubclass<imp::ContainerPriv>)
        @extends Widget,
        @implements gtk4::Orientable;
}

impl Container {
    pub fn new(
        orientation: Orientation,
        _spacing: u32,
    ) -> Self {
        let container: Self = Object::builder().build();
        container.set_orientation(orientation);
        container.set_vexpand(true);
        container.set_hexpand(true);

        container
    }

    pub fn append(&self, child: &impl IsA<Widget>) {
        if let Some(last_child) = self.last_child() {
            let new_separator = self.imp().add_separator();
            new_separator.insert_after(self, Some(&last_child));
            child.insert_after(self, Some(&new_separator));
        } else {
            child.insert_after(self, None::<&Widget>);
        }
    }

    pub fn replace(&self, old: &impl IsA<Widget>, new: &impl IsA<Widget>) {
        new.insert_after(self, Some(old));
        old.unparent();
    }

    pub fn remove(&self, child: &impl IsA<Widget>) {
        // We also need to remove a Separator, if it exists
        let separator = child.next_sibling();
        self.imp().remove_separator(separator);

        // Now remove the child
        child.unparent();
    }

    pub fn children_count(&self) -> usize {
        self.imp().get_children_count()
    }
}

// TODO: Move this to separator?
#[inline]
fn get_opposite_orientation(orientation: Orientation) -> Orientation {
    match orientation {
        Orientation::Horizontal => Orientation::Vertical,
        Orientation::Vertical => Orientation::Horizontal,
        _ => panic!("What the fuck is this orientation"),
    }
}
