use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{gdk::Cursor, GestureDrag, Orientation, Separator as GtkSeparator};
use libadwaita::{glib, prelude::*};

use crate::settings::{SPLIT_HANDLE_WIDTH, SPLIT_VISUAL_WIDTH};

use super::{layout_default::ContainerLayout, layout_tmux::TmuxLayout, Container};

mod imp;

glib::wrapper! {
    pub struct Separator(ObjectSubclass<imp::SeparatorPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

enum Layout {
    Default(ContainerLayout),
    Tmux(TmuxLayout),
}

impl Separator {
    pub fn new(container: &Container, orientation: &Orientation, percentage: f64, handle_size: Option<i32>) -> Self {
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

        // Add ability to drag
        let layout = container.layout_manager().unwrap();
        let layout = match layout.downcast::<ContainerLayout>() {
            Ok(layout) => Layout::Default(layout),
            Err(layout) => {
                let layout: TmuxLayout = layout.downcast().unwrap();
                Layout::Tmux(layout)
            }
        };

        let drag = GestureDrag::new();
        let container = container.clone();
        drag.connect_drag_update(glib::clone!(
            #[strong]
            bin,
            move |drag, offset_x, offset_y| {
                let (start_x, start_y) = drag.start_point().unwrap();
                match &layout {
                    Layout::Default(layout) => layout.drag_update(&container, &bin, start_x + offset_x, start_y + offset_y),
                    Layout::Tmux(layout) => layout.drag_update(&container, &bin, start_x + offset_x, start_y + offset_y),
                }
            }
        ));
        bin.add_controller(drag);

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

    pub fn get_current_position(&self) -> i32 {
        self.imp().current_position.get()
    }

    pub fn set_current_position(&self, pos: i32) {
        self.imp().current_position.replace(pos);
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
