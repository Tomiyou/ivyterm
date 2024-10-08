use std::cell::{Cell, RefCell};

use gtk4::gdk::Cursor;
use gtk4::{Orientation, Separator, Widget};
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, prelude::*, Bin};

use super::layout::IvyLayout;

// Object holding the state
#[derive(Debug, glib::Properties)]
#[properties(wrapper_type = super::IvyPaned)]
pub struct IvyPanedPriv {
    pub separator: RefCell<Option<Bin>>,
    pub separator_visible: Cell<bool>,
    pub start_child: RefCell<Option<Widget>>,
    pub end_child: RefCell<Option<Widget>>,
    #[property(get, set=Self::set_orientation, builder(gtk4::Orientation::Horizontal))]
    orientation: RefCell<gtk4::Orientation>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyPanedPriv {
    const NAME: &'static str = "IvyTerminalCustomPaned";
    type Type = super::IvyPaned;
    type ParentType = Widget;
    type Interfaces = (gtk4::Orientable,);

    fn class_init(gtk_class: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        gtk_class.set_layout_manager_type::<IvyLayout>();
    }

    fn new() -> Self {
        // Here we set the default orientation.
        Self {
            separator: RefCell::new(None),
            separator_visible: Cell::new(false),
            start_child: RefCell::new(None),
            end_child: RefCell::new(None),
            orientation: RefCell::new(Orientation::Horizontal),
        }
    }
}

#[glib::derived_properties]
impl ObjectImpl for IvyPanedPriv {
    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
    }
}

impl WidgetImpl for IvyPanedPriv {}
impl OrientableImpl for IvyPanedPriv {}

impl IvyPanedPriv {
    pub fn set_orientation(&self, orientation: Orientation) {
        self.orientation.replace(orientation);

        let orientation = match orientation {
            Orientation::Horizontal => Orientation::Vertical,
            Orientation::Vertical => Orientation::Horizontal,
            _ => panic!("Unable to invert orientation to create separator"),
        };

        // Create separator widget
        let separator = Separator::new(orientation);
        let separator_container = libadwaita::Bin::builder()
            .child(&separator)
            .css_classes(vec!["separator_bg"])
            .build();
        separator_container.set_parent(self.obj().as_ref());

        // Change the cursor when hovering separator and container
        let cursor = Cursor::from_name("col-resize", None);
        if let Some(cursor) = cursor.as_ref() {
            separator_container.set_cursor(Some(&cursor));
        }

        // Store separator widget inside priv struct
        self.separator.replace(Some(separator_container));
    }
}
