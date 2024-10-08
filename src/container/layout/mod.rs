mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::Orientation;
use libadwaita::{glib, Bin};
use vte4::WidgetExt;

use super::Container;

glib::wrapper! {
    pub struct ContainerLayout(ObjectSubclass<imp::ContainerLayoutPriv>)
        @extends gtk4::LayoutManager;
}

impl ContainerLayout {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn drag_update(&self, paned: &Container, separator: &Bin, x: f64, y: f64) {
        let imp = self.imp();
        let orientation = paned.orientation();

        // Get the handle size
        let (_, handle_size, _, _) = separator.measure(orientation, -1);
        let handle_size = handle_size as f64;
        let handle_half = handle_size / 2.0;

        // First we need to get the size of the first child and size of the handle
        let current_first_child_size = imp.current_first_child_size.get() as f64;

        // TODO: Only queue_allocate() if the actual pixel size changed
        let old_percentage = imp.percentage.get();
        // Offset is counted from the top of separator_container (so take size of the handle)
        // into account
        let (new_percentage, combined_size) = if orientation == Orientation::Horizontal {
            let width = paned.width() as f64;
            let pos = current_first_child_size + handle_half + x;
            let new_percentage = pos / width;
            let combined_size = width - handle_size;

            (new_percentage, combined_size)
        } else {
            let height = paned.height() as f64;
            let pos = current_first_child_size + handle_half + y;
            let new_percentage = pos / height;
            let combined_size = height - handle_size;

            (new_percentage, combined_size)
        };

        let old_pos = (combined_size * old_percentage).round() as i32;
        let new_pos = (combined_size * new_percentage).round() as i32;
        if new_pos != old_pos {
            paned.queue_allocate();

            imp.percentage.replace(new_percentage);
        }
    }
}
