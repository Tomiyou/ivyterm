mod imp;
mod layout_default;
mod layout_tmux;
mod separator;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Type};
use gtk4::{Orientation, Widget};
use imp::Layout;
use libadwaita::{glib, prelude::*};

use crate::{terminal::Terminal, window::IvyWindow};
pub use layout_default::ContainerLayout;
pub use layout_tmux::TmuxLayout;

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

        let layout = if window.is_tmux() {
            let tmux_layout = TmuxLayout::new();
            container.set_layout_manager(Some(tmux_layout.clone()));
            Layout::Tmux(tmux_layout)
        } else {
            let default_layout: ContainerLayout =
                container.layout_manager().unwrap().downcast().unwrap();
            Layout::Default(default_layout)
        };
        imp.layout.replace(Some(layout));

        container
    }

    pub fn append(&self, child: &impl IsA<Widget>, percentage: Option<f64>) {
        if let Some(last_child) = self.last_child() {
            let last_child: Terminal = last_child.downcast().unwrap();
            let layout = self.imp().layout.borrow();
            let new_separator = match layout.as_ref().unwrap() {
                Layout::Default(layout) => layout.add_separator(self),
                Layout::Tmux(layout) => {
                    let char_size = last_child.get_char_width_height();
                    layout.add_separator(self, percentage.unwrap(), char_size)
                },
            };

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

    pub fn remove(&self, child: &impl IsA<Widget>) -> usize {
        let layout = self.imp().layout.borrow();
        let len = match layout.as_ref().unwrap() {
            Layout::Default(layout) => layout.remove_separator(child),
            Layout::Tmux(layout) => layout.remove_separator(),
        };

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
