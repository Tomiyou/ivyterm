mod imp;
mod layout;
mod separator;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Type};
use gtk4::{Orientation, Widget};
use libadwaita::{glib, prelude::*};

pub use layout::TmuxLayout;
pub use separator::TmuxSeparator;

use super::{terminal::TmuxTerminal, IvyTmuxWindow};

glib::wrapper! {
    pub struct TmuxContainer(ObjectSubclass<imp::ContainerPriv>)
        @extends Widget,
        @implements gtk4::Orientable;
}

impl TmuxContainer {
    pub fn new(orientation: &Orientation, window: &IvyTmuxWindow) -> Self {
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
