mod imp;

use glib::Object;
use gtk4::{GestureDrag, Orientation, Widget};
use libadwaita::{glib, prelude::*};
use libadwaita::subclass::prelude::ObjectSubclassIsExt;

use super::{separator::Separator, Container};

glib::wrapper! {
    pub struct TmuxLayout(ObjectSubclass<imp::TmuxLayoutPriv>)
        @extends gtk4::LayoutManager;
}

impl TmuxLayout {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn add_separator(&self, container: &Container, percentage: f64, char_size: (i32, i32)) -> Separator {
        let (char_width, char_height) = char_size;
        // We don't need to calculate the percentages, since Tmux code will do that for us
        let mut separators = self.imp().separators.borrow_mut();

        // Create a new Separator
        let orientation = container.orientation();
        let handle_size = match orientation {
            Orientation::Horizontal => {
                self.imp().char_size.replace(char_size);
                char_width
            },
            _ => {
                self.imp().char_size.replace(char_size);
                char_height
            },
        };
        let separator = Separator::new(&orientation, percentage, Some(handle_size));
        separators.push(separator.clone());

        // // Add ability to drag
        // let drag = GestureDrag::new();
        // drag.connect_drag_update(glib::clone!(
        //     #[strong]
        //     container,
        //     #[strong]
        //     separator,
        //     move |drag, offset_x, offset_y| {
        //         let (start_x, start_y) = drag.start_point().unwrap();
        //         drag_update(
        //             &separator,
        //             &container,
        //             start_x + offset_x,
        //             start_y + offset_y,
        //         );
        //     }
        // ));
        // separator.add_controller(drag);

        separator
    }

    pub fn remove_separator(&self) -> usize {
        todo!()
    }
}

fn drag_update(separator: &Separator, container: &Container, x: f64, y: f64) {}
