use std::cell::Cell;

use gtk4::{Allocation, LayoutManager, Orientation};
use libadwaita::glib;
use libadwaita::subclass::prelude::*;
use vte4::{Cast, WidgetExt};

use crate::paned::IvyPaned;

// Object holding the state
pub struct IvyLayoutPriv {
    pub percentage: Cell<f32>,
    pub last_combined_size: Cell<i32>,
}

impl Default for IvyLayoutPriv {
    fn default() -> Self {
        Self {
            percentage: Cell::new(0.75),
            last_combined_size: Cell::new(-1),
        }
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for IvyLayoutPriv {
    const NAME: &'static str = "MyGtkAppCustomButton";
    type Type = super::IvyLayout;
    type ParentType = LayoutManager;
}

// Trait shared by all GObjects
impl ObjectImpl for IvyLayoutPriv {}

impl LayoutManagerImpl for IvyLayoutPriv {
    fn measure(
        &self,
        widget: &gtk4::Widget,
        orientation: gtk4::Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        let paned: IvyPaned = widget.clone().downcast().unwrap();

        let (minimum, natural) = if orientation == paned.orientation() {
            self.get_preferred_size_for_same_orientation(&paned, orientation, for_size)
        } else {
            self.get_preferred_size_for_opposite_orientation(&paned, orientation, for_size)
        };

        (minimum, natural, -1, -1)
    }

    fn allocate(&self, widget: &gtk4::Widget, width: i32, height: i32, baseline: i32) {
        let paned: IvyPaned = widget.clone().downcast().unwrap();

        let start_child = paned.start_child();
        let start_child_visible =
            start_child.is_some() && start_child.as_ref().unwrap().is_visible();
        let end_child = paned.end_child();
        let end_child_visible = end_child.is_some() && end_child.as_ref().unwrap().is_visible();
        let separator = paned.get_separator();

        if start_child_visible && end_child_visible {
            let start_child = start_child.unwrap();
            let end_child = end_child.unwrap();
            let orientation = paned.orientation();

            let (lala, handle_size, _, _) = separator.measure(orientation, -1);

            let (handle_alloc, start_alloc, end_alloc) = if orientation == Orientation::Horizontal {
                let start_child_width = self.get_start_child_size(width - handle_size);
                let handle_allocation = Allocation::new(start_child_width, 0, handle_size, height);
                let start_child_allocation = Allocation::new(0, 0, start_child_width, height);
                let end_child_allocation = Allocation::new(
                    start_child_width + handle_size,
                    0,
                    width - start_child_width - handle_size,
                    height,
                );

                (
                    handle_allocation,
                    start_child_allocation,
                    end_child_allocation,
                )
            } else {
                let start_child_height = self.get_start_child_size(height - handle_size);
                let handle_allocation = Allocation::new(0, start_child_height, width, handle_size);
                let start_child_allocation = Allocation::new(0, 0, width, start_child_height);
                let end_child_allocation = Allocation::new(
                    0,
                    start_child_height + handle_size,
                    width,
                    height - start_child_height - handle_size,
                );

                (
                    handle_allocation,
                    start_child_allocation,
                    end_child_allocation,
                )
            };

            separator.set_child_visible(true);
            separator.size_allocate(&handle_alloc, -1);
            start_child.size_allocate(&start_alloc, -1);
            end_child.size_allocate(&end_alloc, -1);
        } else {
            let allocation = Allocation::new(0, 0, width, height);
            if start_child_visible {
                let start_child = start_child.unwrap();
                start_child.set_child_visible(true);
                start_child.size_allocate(&allocation, -1);
            } else if end_child_visible {
                let end_child = end_child.unwrap();
                end_child.set_child_visible(true);
                end_child.size_allocate(&allocation, -1);
            }

            separator.set_child_visible(false);
        }
    }

    fn create_layout_child(
        &self,
        widget: &gtk4::Widget,
        for_child: &gtk4::Widget,
    ) -> gtk4::LayoutChild {
        println!("create_layout_child widget {:p}", widget);
        self.parent_create_layout_child(widget, for_child)
    }
}

impl IvyLayoutPriv {
    fn get_preferred_size_for_opposite_orientation(
        &self,
        paned: &IvyPaned,
        opposite_orientation: gtk4::Orientation,
        size: i32,
    ) -> (i32, i32) {
        let separator = paned.get_separator();
        let start_child = paned.start_child();
        let end_child = paned.end_child();
        let mut separator_visible = false;
        let orientation = get_opposite_orientation(opposite_orientation);

        // `Size` is the size of the widget in the orientation of the Paned widget. We need to take
        // percentage into account if we want to calculate the correct opposite size.
        let (for_start_child, for_end_child, for_handle) = if size > -1 {
            let start_child = start_child.as_ref().unwrap();
            let end_child = end_child.as_ref().unwrap();

            if start_child.is_visible() && end_child.is_visible() {
                separator_visible = true;

                let (_, handle_size, _, _) = separator.measure(orientation, -1);

                let start_child_size = self.get_start_child_size(size - handle_size);
                let end_child_size = size - start_child_size - handle_size;

                (start_child_size, end_child_size, handle_size)
            } else {
                (size, size, -1)
            }
        } else {
            (size, size, -1)
        };

        let mut minimum = 0;
        let mut natural = 0;

        // Now measure children in the opposite orientation of the Paned widget
        if let Some(start_child) = start_child {
            if start_child.is_visible() {
                let (child_min, child_nat, _, _) =
                    start_child.measure(opposite_orientation, for_start_child);
                minimum = child_min;
                natural = child_nat;
            }
        }

        if let Some(end_child) = end_child {
            if end_child.is_visible() {
                let (child_min, child_nat, _, _) =
                    end_child.measure(opposite_orientation, for_end_child);
                minimum = minimum.max(child_min);
                natural = natural.max(child_nat);
            }
        }

        if separator_visible {
            let (child_min, child_nat, _, _) = separator.measure(opposite_orientation, for_handle);
            minimum = minimum.max(child_min);
            natural = natural.max(child_nat);
        }

        (minimum, natural)
    }

    fn get_preferred_size_for_same_orientation(
        &self,
        paned: &IvyPaned,
        orientation: gtk4::Orientation,
        for_size: i32,
    ) -> (i32, i32) {
        let mut minimum = 0;
        let mut natural = 0;

        let start_child = paned.start_child();
        let end_child = paned.end_child();
        let mut separator_visible = false;

        // We are given size in the opposite orientation as the widget and have the calculate
        // minimum and preferred size for the same orientation as the widget. This is easy, since
        // all children would be the same opposite orientation size, in other words, children all
        // have the same size in opposite orientation. Percentage does not affect it.

        if let Some(start_child) = start_child {
            if start_child.is_visible() {
                let (child_min, child_nat, _, _) = start_child.measure(orientation, for_size);
                minimum = child_min;
                natural = child_nat;
                separator_visible = true;
            }
        }

        if let Some(end_child) = end_child {
            if end_child.is_visible() {
                let (child_min, child_nat, _, _) = end_child.measure(orientation, for_size);
                minimum += child_min;
                natural += child_nat;
                separator_visible = separator_visible && true;
            }
        }

        let separator = paned.get_separator();
        if separator_visible {
            let (_, handle_size, _, _) = separator.measure(orientation, -1);
            minimum += handle_size;
            natural += handle_size;
        }

        (minimum, natural)
    }

    #[inline]
    fn get_start_child_size(&self, combined_size: i32) -> i32 {
        (self.percentage.get() * combined_size as f32) as i32
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
