mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{gdk::Cursor, Box, Orientation, Separator, Widget};
use libadwaita::{glib, prelude::*};

glib::wrapper! {
    pub struct IvyPaned(ObjectSubclass<imp::IvyPanedPriv>)
        @extends Box, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl IvyPaned {
    pub fn new(
        orientation: Orientation,
        start_child: impl IsA<Widget>,
        end_child: impl IsA<Widget>,
    ) -> Self {
        let container: Self = Object::builder().build();
        container.set_spacing(0);
        container.set_orientation(orientation);

        {
            let separator = Separator::new(Orientation::Vertical);
            let bin = libadwaita::Bin::builder().child(&separator).css_classes(vec!["separator_bg"]).build();

            let cursor = Cursor::from_name("col-resize", None);
            if let Some(cursor) = cursor.as_ref() {
                bin.set_cursor(Some(&cursor));
            }

            container.imp().separator.borrow_mut().replace(bin);
        };

        container.set_start_child(Some(&start_child));
        container.set_end_child(Some(&end_child));

        container
    }

    fn show_separator(&self, prepend: bool) {
        let imp = self.imp();
        if imp.separator_visible.get() == true {
            // No need to show separator if it is already visible
            return
        }

        // Show separator
        let binding = imp.separator.borrow();
        let separator = binding.as_ref().unwrap();
        if prepend {
            self.prepend(separator);
        } else {
            self.append(separator);
        }
    }

    fn hide_separator(&self) {
        let imp = self.imp();
        if imp.separator_visible.get() == false {
            // No need to hide separator if it is already hidden
            return
        }

        imp.separator_visible.replace(false);
        self.remove(imp.separator.borrow().as_ref().unwrap());
    }

    pub fn set_start_child(&self, new_child: Option<&impl IsA<Widget>>) {
        let imp = self.imp();
        let mut start_child = imp.start_child.borrow_mut();
        let end_child = imp.end_child.borrow();

        if let Some(old_child) = start_child.take() {
            // Remove child from box container
            self.remove(&old_child);
        }

        if let Some(new_child) = new_child {
            // If both end_child and start_child are Some(), we should show separator
            if end_child.is_some() {
                self.show_separator(true);
            }

            // Set start child
            start_child.replace(new_child.clone().into());
            // Add child to box container
            self.prepend(new_child);
        } else {
            self.hide_separator();
        }
    }

    pub fn set_end_child(&self, new_child: Option<&impl IsA<Widget>>) {
        let imp = self.imp();
        let start_child = imp.start_child.borrow();
        let mut end_child = imp.end_child.borrow_mut();

        if let Some(old_child) = end_child.take() {
            // Remove child from box container
            self.remove(&old_child);
        }

        if let Some(new_child) = new_child {
            // If both end_child and start_child are Some(), we should show separator
            if start_child.is_some() {
                self.show_separator(false);
            }

            // Set end child
            end_child.replace(new_child.clone().into());
            // Add child to box container
            self.append(new_child);
        } else {
            self.hide_separator();
        }
    }

    pub fn get_start_child(&self) -> Option<Widget> {
        self.imp().start_child.borrow().clone()
    }

    pub fn get_end_child(&self) -> Option<Widget> {
        self.imp().end_child.borrow().clone()
    }
}
