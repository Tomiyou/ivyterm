mod imp;

use glib::Object;
use gtk4::Orientation;
use libadwaita::glib;
use vte4::WidgetExt;

use super::{separator::Separator, Container};

glib::wrapper! {
    pub struct ContainerLayout(ObjectSubclass<imp::ContainerLayoutPriv>)
        @extends gtk4::LayoutManager;
}

impl ContainerLayout {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn drag_update(&self, container: &Container, separator: &Separator, x: f64, y: f64) {
        let orientation = container.orientation();

        // Get the handle size
        let current_position = separator.get_current_position();
        let handle_size = separator.get_handle_width() as f64;
        let handle_half = handle_size * 0.5;

        // println!("Drag x: {}, y: {}", x, y);

        let (new_position, percentage) = if orientation == Orientation::Horizontal {
            let pos = current_position as f64 + x - 2.0;
            let width = container.width() as f64;
            let percentage = (pos + handle_half) / width;

            (pos.round() as i32, percentage)
        } else {
            let pos = current_position as f64 + y - 2.0;
            let height = container.height() as f64;
            let percentage = (pos + handle_half) / height;

            (pos.round() as i32, percentage)
        };

        // println!("drag_update: Old {} vs. new {} | {}", current_position, new_position, percentage);

        if new_position != current_position {
            container.queue_allocate();
            separator.set_percentage(percentage);
        }
    }
}
