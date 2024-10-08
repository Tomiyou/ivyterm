mod imp;
mod layout_tmux;
mod separator;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Type};
use gtk4::{Orientation, Widget};
use libadwaita::{glib, prelude::*};

pub use layout_tmux::TmuxLayout;
pub use separator::TmuxSeparator;

use super::{terminal::TmuxTerminal, IvyTmuxWindow};

glib::wrapper! {
    pub struct TmuxContainer(ObjectSubclass<imp::ContainerPriv>)
        @extends Widget,
        @implements gtk4::Orientable;
}

impl TmuxContainer {
    pub fn new(orientation: Orientation, window: &IvyTmuxWindow) -> Self {
        let container: Self = Object::builder().build();
        container.set_orientation(orientation);
        container.set_vexpand(true);
        container.set_hexpand(true);

        let imp = container.imp();
        imp.window.replace(Some(window.clone()));

        let layout = TmuxLayout::new();
        container.set_layout_manager(Some(layout.clone()));
        imp.layout.replace(Some(layout));

        container
    }

    fn create_separator(&self, position: i32) -> TmuxSeparator {
        let imp = self.imp();
        let binding = imp.window.borrow();
        let window = binding.as_ref().unwrap();
        let char_size = window.get_char_size();
        let binding = imp.layout.borrow();
        let layout = binding.as_ref().unwrap();
        layout.add_separator(self, position, char_size)
    }

    pub fn append(&self, child: &impl IsA<Widget>, position: i32) {
        if let Some(last_child) = self.last_child() {
            let new_separator = self.create_separator(position);
            new_separator.insert_after(self, Some(&last_child));
            child.insert_after(self, Some(&new_separator));
        } else {
            child.insert_after(self, None::<&Widget>);
        }
    }

    pub fn prepend(&self, child: &impl IsA<Widget>, sibling: &Option<impl IsA<Widget>>, position: i32) {
        // TODO: Prepend on sibling None means append() last...
        if let Some(sibling) = sibling {
            let new_separator = self.create_separator(position);
            child.insert_before(self, Some(sibling));
            new_separator.insert_after(self, Some(child));
        } else {
            self.append(child, position);
        }
    }

    pub fn replace(&self, old: &impl IsA<Widget>, new: &impl IsA<Widget>) {
        new.insert_after(self, Some(old));
        old.unparent();
    }

    pub fn remove(&self, child: &impl IsA<Widget>) {
        // First try and remove the associated separator
        if let Some(separator) = child.next_sibling() {
            separator.unparent();
        } else if let Some(separator) = child.prev_sibling() {
            separator.unparent();
        }

        // Now remove the child
        child.unparent();
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
                let terminal: TmuxTerminal = child.clone().downcast().unwrap();
                terminal.get_cols_or_rows()
            } else if child_type == container_type {
                let container: TmuxContainer = child.clone().downcast().unwrap();
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
