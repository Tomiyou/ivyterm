use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{gdk::Cursor, GestureDrag, Orientation, Separator as GtkSeparator};
use libadwaita::{glib, prelude::*};
use log::debug;

use crate::config::SPLIT_VISUAL_WIDTH;

use super::IvyTmuxWindow;

mod imp;

glib::wrapper! {
    pub struct TmuxSeparator(ObjectSubclass<imp::SeparatorPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl TmuxSeparator {
    pub fn new(orientation: &Orientation, handle_size: i32, position: i32) -> Self {
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

        // Calculate Handle size and apply margins
        // Since each VTE widget also has a fixed padding of 1 px for each direction,
        // we subtract 2
        let handle_size = handle_size - 2;
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

        bin.set_position(position);

        // Change the cursor when hovering separator and container
        let cursor = Cursor::from_name(cursor, None);
        if let Some(cursor) = cursor.as_ref() {
            bin.set_cursor(Some(&cursor));
        }

        // Add ability to drag
        let drag = GestureDrag::new();
        drag.connect_drag_begin(glib::clone!(
            #[weak]
            bin,
            move |_, _, _| {
                let imp = bin.imp();
                let position = imp.position.get();
                imp.position_before_drag.replace(position);
            }
        ));
        drag.connect_drag_update(glib::clone!(
            #[weak]
            bin,
            move |_, offset_x, offset_y| {
                drag_update(&bin, offset_x, offset_y);
            }
        ));
        drag.connect_drag_end(glib::clone!(
            #[weak]
            bin,
            move |drag, _, _| {
                if let Some(window) = drag.widget().unwrap().root() {
                    if let Ok(window) = window.downcast::<IvyTmuxWindow>() {
                        let imp = bin.imp();
                        let old_position = imp.position_before_drag.get();
                        let new_position = imp.position.get();
                        if new_position == old_position {
                            return;
                        }
                        // Sync the change back to Tmux
                        window.separator_drag_sync(&bin, new_position - old_position);
                    }
                }
            }
        ));
        bin.add_controller(drag);

        bin
    }

    pub fn get_handle_width(&self) -> i32 {
        let orientation = get_opposite_orientation(self.orientation());
        let (_, handle_size, _, _) = self.measure(orientation, -1);
        handle_size
    }

    pub fn get_position(&self) -> i32 {
        self.imp().position.get()
    }

    pub fn set_position(&self, new: i32) {
        self.imp().position.replace(new);
    }

    pub fn adjust_position(&self, factor: f64) {
        let position = &self.imp().position;
        let old = position.get() as f64;
        let new = old * factor;
        position.replace(new as i32);
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

#[inline]
fn drag_update(separator: &TmuxSeparator, offset_x: f64, offset_y: f64) {
    let old_position = separator.get_position();
    // Account for 1px of internal padding
    let handle_width = (separator.get_handle_width() + 2) as f64;

    let new_position = match separator.orientation() {
        Orientation::Vertical => {
            let offset = offset_x / handle_width;
            old_position + (offset.round() as i32)
        }
        _ => {
            let offset = offset_y / handle_width;
            old_position + (offset.round() as i32)
        }
    };

    // If the position did not change, we can stop now
    if new_position == old_position {
        return;
    }

    // We need to check that this new position does not overlap with previous Separator
    let prev_separator = separator.prev_sibling().unwrap().prev_sibling();
    if let Some(prev_separator) = prev_separator {
        let prev_separator: TmuxSeparator = prev_separator.downcast().unwrap();
        if new_position < prev_separator.get_position() + 2 {
            return;
        }
    } else {
        // We are the first separator
        if new_position < 1 {
            return;
        }
    }

    // We need to check that this new position does not overlap with next Separator
    let next_separator = separator.next_sibling().unwrap().next_sibling();
    // We remember parent as an optimization
    let parent = if let Some(next_separator) = next_separator {
        let next_separator: TmuxSeparator = next_separator.downcast().unwrap();
        if new_position > next_separator.get_position() - 2 {
            return;
        }
        separator.parent().unwrap()
    } else {
        // We are the last separator, ensure we don't overlap the parent
        let parent = separator.parent().unwrap();
        let allocation = parent.allocation();
        let parent_size = match separator.orientation() {
            Orientation::Vertical => allocation.width() as f64 / handle_width,
            _ => allocation.height() as f64 / handle_width,
        };
        let parent_size = parent_size.round() as i32;

        if new_position > parent_size - 2 {
            return;
        }

        parent
    };

    debug!("TmuxSeparator: drag to new_position {}", new_position);
    separator.set_position(new_position);
    parent.queue_allocate();
}
