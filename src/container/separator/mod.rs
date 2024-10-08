use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{Orientation, Separator as GtkSeparator};
use libadwaita::{glib, prelude::*};

use crate::container::get_opposite_orientation;

mod imp;

glib::wrapper! {
    pub struct Separator(ObjectSubclass<imp::SeparatorPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Separator {
    pub fn new(orientation: &Orientation, percentage: f64) -> Self {
        let (separator_orientation, css_class, cursor) = match orientation {
            Orientation::Horizontal => (
                Orientation::Vertical,
                "separator_cont_vertical",
                "col-resize",
            ),
            Orientation::Vertical => (
                Orientation::Horizontal,
                "separator_cont_horizontal",
                "row-resize",
            ),
            _ => panic!("Unable to invert orientation to create separator"),
        };

        // Create separator widget
        let separator = GtkSeparator::new(separator_orientation);

        let bin: Self = Object::builder().build();
        bin.set_orientation(separator_orientation);
        bin.set_child(Some(&separator));
        bin.set_css_classes(&[css_class]);
        bin.set_percentage(percentage);

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
}
