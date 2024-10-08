use std::cell::{Cell, RefCell};

use gtk4::gdk::Cursor;
use gtk4::{GestureDrag, Orientation, Widget};
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, prelude::*, Bin};

use super::layout::ContainerLayout;
use super::separator::Separator;

// Object holding the state
#[derive(Debug, glib::Properties)]
#[properties(wrapper_type = super::Container)]
pub struct ContainerPriv {
    separators: RefCell<Vec<Separator>>,
    #[property(get, set=Self::set_orientation, builder(gtk4::Orientation::Horizontal))]
    orientation: RefCell<gtk4::Orientation>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ContainerPriv {
    const NAME: &'static str = "IvyTerminalContainer";
    type Type = super::Container;
    type ParentType = Widget;
    type Interfaces = (gtk4::Orientable,);

    fn class_init(gtk_class: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        gtk_class.set_layout_manager_type::<ContainerLayout>();
    }

    fn new() -> Self {
        // Here we set the default orientation.
        Self {
            separators: RefCell::new(Vec::new()),
            orientation: RefCell::new(Orientation::Horizontal),
        }
    }
}

#[glib::derived_properties]
impl ObjectImpl for ContainerPriv {
    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
    }
}

impl WidgetImpl for ContainerPriv {}
impl OrientableImpl for ContainerPriv {}

impl ContainerPriv {
    pub fn get_children_count(&self) -> usize {
        self.separators.borrow().len() + 1
    }

    pub fn add_separator(&self) -> Separator {
        let mut separators = self.separators.borrow_mut();
        // There is always 1 more child than there is separators
        let old_len = separators.len() + 1;
        let new_len = old_len + 1;

        // Update percentages of already existing Separators
        let new_percentage = old_len as f64 / new_len as f64;
        for separator in separators.iter_mut() {
            let percentage = separator.get_percentage() * new_percentage;
            separator.set_percentage(percentage);
        }

        // Create a new Separator
        let orientation = self.orientation.borrow();
        let separator = Separator::new(&orientation, 1.0 - new_percentage);
        separators.push(separator.clone());

        separator
    }

    pub fn remove_separator(&self, removed: Option<Widget>) {
        let mut separators = self.separators.borrow_mut();
        let old_len = separators.len() + 1;
        let new_len = old_len - 1;

        let (removed, percentage) = if let Some(removed) = removed {
            let removed: Separator = removed.downcast().unwrap();
            let removed_percentage = removed.get_percentage();
            (removed, removed_percentage)
        } else {
            // Last child was removed, special case
            let removed_percentage = 1.0 - separators
                .iter()
                .fold(0.0, |acc, separator| acc + separator.get_percentage());
            let removed = separators.pop().unwrap();
            (removed, removed_percentage)
        };

        // Distribute the removed percentage between retained ones
        let distributed = percentage / new_len as f64;
        separators.retain(|separator| {
            if separator.eq(&removed) {
                return false;
            }

            let percentage = separator.get_percentage();
            separator.set_percentage(percentage + distributed);
            true
        });

        removed.unparent();
    }

    pub fn set_orientation(&self, orientation: Orientation) {
        self.orientation.replace(orientation);

        // let (separator_orientation, css_class, cursor) = match orientation {
        //     Orientation::Horizontal => (
        //         Orientation::Vertical,
        //         "separator_cont_vertical",
        //         "col-resize",
        //     ),
        //     Orientation::Vertical => (
        //         Orientation::Horizontal,
        //         "separator_cont_horizontal",
        //         "row-resize",
        //     ),
        //     _ => panic!("Unable to invert orientation to create separator"),
        // };

        // // Create separator widget
        // let separator = Separator::new(separator_orientation);
        // let separator_container = libadwaita::Bin::builder()
        //     .child(&separator)
        //     .css_classes(vec![css_class])
        //     .build();
        // separator_container.set_parent(self.obj().as_ref());

        // // Change the cursor when hovering separator and container
        // let cursor = Cursor::from_name(cursor, None);
        // if let Some(cursor) = cursor.as_ref() {
        //     separator_container.set_cursor(Some(&cursor));
        // }

        // // Add ability to drag
        // let drag = GestureDrag::new();
        // let paned = self.obj().clone();
        // let layout: ContainerLayout = paned.layout_manager().unwrap().downcast().unwrap();
        // let _separator_container = separator_container.clone();
        // drag.connect_drag_update(move |drag, offset_x, offset_y| {
        //     let (start_x, start_y) = drag.start_point().unwrap();
        //     layout.drag_update(&paned, &_separator_container, start_x + offset_x, start_y + offset_y);
        // });
        // separator_container.add_controller(drag);

        // // Store separator widget inside priv struct
        // self.separator.replace(Some(separator_container));
    }
}
