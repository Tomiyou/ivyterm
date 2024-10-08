mod imp;
mod layout_default;
mod layout_tmux;
mod separator;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Type};
use gtk4::{Orientation, Widget};
use layout_tmux::TmuxLayout;
use libadwaita::{glib, prelude::*};

use crate::{terminal::Terminal, window::IvyWindow};

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

        container.imp().window.replace(Some(window.clone()));

        if window.is_tmux() {
            let tmux_layout = TmuxLayout::new();
            container.set_layout_manager(Some(tmux_layout));
        }

        container
    }

    pub fn append(&self, child: &impl IsA<Widget>) {
        if let Some(last_child) = self.last_child() {
            let last_child: Terminal = last_child.downcast().unwrap();

            // If tmux session is active, Separator's width should be the same as Terminal char width/height
            let binding = self.imp().window.borrow();
            let window = binding.as_ref().unwrap();
            let handle_size = if window.is_tmux() {
                let orientation = self.orientation();
                let (char_width, char_height) = last_child.get_char_width_height();

                match orientation {
                    Orientation::Horizontal => Some(char_width as i32),
                    _ => Some(char_height as i32),
                }
            } else {
                None
            };

            let new_separator = self.imp().add_separator(handle_size);
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
