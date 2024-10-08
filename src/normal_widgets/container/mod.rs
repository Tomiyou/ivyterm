mod imp;
mod layout_default;
mod separator;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Type};
use gtk4::{Orientation, Widget};
use libadwaita::{glib, prelude::*};

pub use layout_default::ContainerLayout;

use super::{terminal::Terminal, window::IvyWindow};

glib::wrapper! {
    pub struct Container(ObjectSubclass<imp::ContainerPriv>)
        @extends Widget,
        @implements gtk4::Orientable;
}

impl Container {
    pub fn new(orientation: Orientation, window: &IvyWindow) -> Self {
        let container: Self = Object::builder().build();
        container.set_orientation(orientation);
        container.set_vexpand(true);
        container.set_hexpand(true);

        let imp = container.imp();
        imp.window.replace(Some(window.clone()));

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

    pub fn recursive_size_measure(&self) -> (i64, i64) {
        let container_type = Type::from_name("IvyTerminalContainer").unwrap();
        let terminal_type = Type::from_name("IvyTerminalCustomTerminal").unwrap();
        let parent_orientation = self.orientation();

        let (mut total_cols, mut total_rows) = if parent_orientation == Orientation::Horizontal {
            (0, i64::MAX)
        } else {
            (i64::MAX, 0)
        };

        let mut next_child = self.first_child();
        while let Some(child) = next_child {
            let child_type = child.type_();

            let (cols, rows) = if child_type == terminal_type {
                let terminal: Terminal = child.clone().downcast().unwrap();
                terminal.get_cols_or_rows()
            } else if child_type == container_type {
                let container: Container = child.clone().downcast().unwrap();
                container.recursive_size_measure()
            } else {
                // Skip children of type Separator
                if parent_orientation == Orientation::Horizontal {
                    total_cols += 1;
                } else {
                    total_rows += 1;
                }

                next_child = child.next_sibling();
                continue;
            };

            if parent_orientation == Orientation::Horizontal {
                total_cols += cols;
                total_rows = total_rows.min(rows);
            } else {
                total_cols = total_cols.min(cols);
                total_rows += rows;
            }

            next_child = child.next_sibling();
        }

        (total_cols, total_rows)
    }
}
