mod imp;

use glib::Object;
use gtk4::{GestureDrag, Orientation, Widget};
use libadwaita::subclass::prelude::ObjectSubclassIsExt;
use libadwaita::{glib, prelude::*};

use super::separator::Separator;
use super::Container;

glib::wrapper! {
    pub struct ContainerLayout(ObjectSubclass<imp::ContainerLayoutPriv>)
        @extends gtk4::LayoutManager;
}

impl ContainerLayout {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn add_separator(&self, container: &Container) -> Separator {
        let mut separators = self.imp().separators.borrow_mut();
        // There is always 1 more child than there is separators
        let old_len = separators.len() + 1;
        let new_len = old_len + 1;

        // Update percentages of already existing Separators
        let new_percentage = old_len as f64 / new_len as f64;
        for separator in separators.iter_mut() {
            let old = separator.get_percentage();
            separator.set_percentage(old * new_percentage);
        }

        // Create a new Separator
        let orientation = container.orientation();
        let separator = Separator::new(&orientation, new_percentage);
        separators.push(separator.clone());

        // Add ability to drag
        let drag = GestureDrag::new();
        drag.connect_drag_update(glib::clone!(
            #[strong]
            container,
            #[strong]
            separator,
            #[strong(rename_to = layout)]
            self,
            move |drag, offset_x, offset_y| {
                let (start_x, start_y) = drag.start_point().unwrap();
                layout.drag_update(
                    &separator,
                    &container,
                    start_x + offset_x,
                    start_y + offset_y,
                );
            }
        ));
        separator.add_controller(drag);

        separator
    }

    pub fn remove_separator(&self, separator: Option<Widget>) -> usize {
        let mut separators = self.imp().separators.borrow_mut();

        // Find the removed Separator
        let (removed_separator, removed_percentage) = if let Some(separator) = separator {
            let removed_separator: Separator = separator.downcast().unwrap();
            let mut previous_percentage = 0.0;
            let mut removed_percentage = 0.0;
            for separator in separators.iter() {
                let current_percentage = separator.get_percentage();
                if separator.eq(&removed_separator) {
                    removed_percentage = current_percentage - previous_percentage;
                    break;
                }
                previous_percentage = current_percentage;
            }
            (removed_separator, removed_percentage)
        } else {
            // Last pane is removed
            let separator = separators.pop().unwrap();
            let removed_percentage = 1.0 - separator.get_percentage();
            (separator, removed_percentage)
        };

        let opposite = 1.0 - removed_percentage;

        // Distribute the removed size
        let mut found = false;
        separators.retain_mut(|separator| {
            if removed_separator.eq(separator) {
                found = true;
                return false;
            }

            let old_percentage = separator.get_percentage();
            let new_percentage = if found {
                (old_percentage - removed_percentage) / opposite
            } else {
                old_percentage / opposite
            };
            separator.set_percentage(new_percentage);
            println!("New percentage {}", new_percentage);

            true
        });

        removed_separator.unparent();

        separators.len() + 1
    }

    fn drag_update(&self, separator: &Separator, container: &Container, x: f64, y: f64) {
        let orientation = container.orientation();
        let allocation = separator.allocation();

        if orientation == Orientation::Horizontal {
            let old_position = allocation.x();
            let new_position = old_position as f64 + x;
            let new_position = new_position.round() as i32;

            if new_position != old_position {
                let container_width = container.allocation().width();
                let new_percentage = new_position as f64 / container_width as f64;
                // println!("X Position {} -> {} | percentage: {}", old_position, new_position, percentage);
                separator.set_percentage(new_percentage);
                container.queue_allocate();
            }
        } else {
            let old_position = allocation.y();
            let new_position = old_position as f64 + y;
            let new_position = new_position.round() as i32;

            if new_position != old_position {
                let container_height = container.allocation().height();
                let new_percentage = new_position as f64 / container_height as f64;
                // println!("Y Position {} -> {} | percentage: {}", old_position, new_position, percentage);
                separator.set_percentage(new_percentage);
                container.queue_allocate();
            }
        };
    }
}
