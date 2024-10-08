use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{gdk::Cursor, Orientation, Separator as GtkSeparator};
use libadwaita::{glib, prelude::*};

use crate::settings::{SPLIT_HANDLE_WIDTH, SPLIT_VISUAL_WIDTH};

mod imp;

glib::wrapper! {
    pub struct Separator(ObjectSubclass<imp::SeparatorPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Separator {
    pub fn new(
        orientation: &Orientation,
        percentage: f64,
        handle_size: Option<i32>,
    ) -> Self {
        let (separator_orientation, cursor) = match orientation {
            Orientation::Horizontal => (Orientation::Vertical, "col-resize"),
            Orientation::Vertical => (Orientation::Horizontal, "row-resize"),
            _ => panic!("Unable to invert orientation to create separator"),
        };

        // Create separator widget
        let separator = GtkSeparator::new(separator_orientation);

        let bin: Self = Object::builder().build();
        bin.set_orientation(separator_orientation);
        bin.set_child(Some(&separator));
        bin.set_css_classes(&["separator_container"]);
        bin.set_percentage(percentage);

        // Calculate Handle size and apply margins
        // Since each VTE widget also has a fixed padding of 1 px for each direction,
        // we subtract 2
        let handle_size = handle_size.unwrap_or(SPLIT_HANDLE_WIDTH) - 2;
        let margin_size = handle_size - SPLIT_VISUAL_WIDTH;
        let first_half = margin_size / 2;
        let second_half = margin_size - first_half;
        if separator_orientation == Orientation::Horizontal {
            separator.set_margin_top(first_half);
            separator.set_margin_bottom(second_half);
        } else {
            separator.set_margin_start(first_half);
            separator.set_margin_end(second_half);
        }

        // Change the cursor when hovering separator and container
        let cursor = Cursor::from_name(cursor, None);
        if let Some(cursor) = cursor.as_ref() {
            bin.set_cursor(Some(&cursor));
        }

        bin
    }

    pub fn get_percentage(&self) -> f64 {
        self.imp().percentage.get()
    }

    pub fn set_percentage(&self, percentage: f64) -> f64 {
        self.imp().percentage.replace(percentage)
    }

    pub fn get_handle_width(&self) -> i32 {
        let orientation = get_opposite_orientation(self.orientation());
        let (_, handle_size, _, _) = self.measure(orientation, -1);
        handle_size
    }

    pub fn get_prev_sibling_size(&self) -> i32 {
        self.imp().prev_sibling_size.get()
    }

    pub fn set_prev_sibling_size(&self, pos: i32) {
        self.imp().prev_sibling_size.replace(pos);
    }
}

#[inline]
fn get_opposite_orientation(orientation: Orientation) -> Orientation {
    match orientation {
        Orientation::Horizontal => Orientation::Vertical,
        Orientation::Vertical => Orientation::Horizontal,
        _ => panic!("What the fuck is this orientation"),
    }
}
