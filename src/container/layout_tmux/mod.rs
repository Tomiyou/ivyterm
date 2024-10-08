mod imp;

use glib::Object;
use gtk4::{GestureDrag, Orientation, Widget};
use imp::TmuxSeparator;
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

    pub fn add_separator(&self, container: &Container, position: i32, char_size: (i32, i32)) -> Separator {
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
        let separator = Separator::new(&orientation, Some(handle_size));
        separators.push(TmuxSeparator {
            s: separator.clone(),
            position: position as i32,
        });

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

    pub fn set_separator_position(&self, target: &impl IsA<Widget>, position: i32) {
        let mut separators = self.imp().separators.borrow_mut();
        for separator in separators.iter_mut() {
            if separator.s.eq(target) {
                println!("Replacing separator position {} -> {}", separator.position, position);
                separator.position = position;
                break;
            }
        }
    }
}

fn drag_update(separator: &Separator, container: &Container, x: f64, y: f64) {}
