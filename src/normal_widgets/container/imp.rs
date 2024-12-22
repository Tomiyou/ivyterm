use std::cell::RefCell;

use gtk4::{Orientation, Widget};
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, prelude::*};

use super::layout::ContainerLayout;

// Object holding the state
#[derive(glib::Properties)]
#[properties(wrapper_type = super::Container)]
pub struct ContainerPriv {
    pub layout: RefCell<Option<ContainerLayout>>,
    #[property(get, set=Self::set_orientation, builder(gtk4::Orientation::Horizontal))]
    orientation: RefCell<gtk4::Orientation>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ContainerPriv {
    const NAME: &'static str = "ivytermContainer";
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
            layout: RefCell::new(None),
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
        self.layout.take();
    }
}

impl WidgetImpl for ContainerPriv {}
impl OrientableImpl for ContainerPriv {}

impl ContainerPriv {
    pub fn set_orientation(&self, orientation: Orientation) {
        self.orientation.replace(orientation);
    }
}
