use std::cell::Cell;

use gtk4::{Allocation, LayoutManager, Orientation};
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, Bin};
use vte4::{Cast, WidgetExt};

use crate::container::separator::Separator;
use crate::container::Container;

// Object holding the state
pub struct ContainerLayoutPriv {
    pub percentage: Cell<f64>,
    pub last_combined_size: Cell<i32>,
    pub current_first_child_size: Cell<i32>,
}

impl Default for ContainerLayoutPriv {
    fn default() -> Self {
        Self {
            percentage: Cell::new(0.5),
            last_combined_size: Cell::new(-1),
            current_first_child_size: Cell::new(-1),
        }
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ContainerLayoutPriv {
    const NAME: &'static str = "MyGtkAppCustomButton";
    type Type = super::ContainerLayout;
    type ParentType = LayoutManager;
}

// Trait shared by all GObjects
impl ObjectImpl for ContainerLayoutPriv {}

impl LayoutManagerImpl for ContainerLayoutPriv {
    fn measure(
        &self,
        widget: &gtk4::Widget,
        orientation: gtk4::Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        let paned: Container = widget.clone().downcast().unwrap();

        let (minimum, natural) = if orientation == paned.orientation() {
            self.get_preferred_size_for_same_orientation(&paned, orientation, for_size)
        } else {
            self.get_preferred_size_for_opposite_orientation(&paned, orientation, for_size)
        };

        (minimum, natural, -1, -1)
    }

    fn allocate(&self, widget: &gtk4::Widget, width: i32, height: i32, _baseline: i32) {
        let paned: Container = widget.clone().downcast().unwrap();
        let orientation = paned.orientation();

        let allocations: Vec<Allocation> = if orientation == Orientation::Horizontal {
            let children_sizes = self.get_children_sizes(&paned, width);

            let mut x = 0;
            children_sizes.iter().map(|child_width| {
                let allocation = Allocation::new(x, 0, *child_width, height);
                x += child_width;

                allocation
            }).collect()
        } else {
            let children_sizes = self.get_children_sizes(&paned, height);

            let mut y = 0;
            children_sizes.iter().map(|child_height| {
                let allocation = Allocation::new(0, y, width, *child_height);
                y += child_height;

                allocation
            }).collect()
        };

        let mut i = 0;
        let mut next_child = paned.first_child();
        while let Some(child) = next_child {
            let allocation = allocations[i];
            child.size_allocate(&allocation, -1);

            next_child = child.next_sibling();
            i += 1;
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

impl ContainerLayoutPriv {
    fn get_preferred_size_for_opposite_orientation(
        &self,
        container: &Container,
        opposite_orientation: gtk4::Orientation,
        size: i32,
    ) -> (i32, i32) {
        // For a container like this [   |   ] (vertical split), this means we are given width of
        // entire container and need to calculate height.
        // But since we need to measure height of each child, we need to calculate width of each
        // child, which depends on percentage of each split.
        let children_sizes = self.get_children_sizes(container, size);

        let mut minimum = 0;
        let mut natural = 0;

        let mut i = 0;
        let mut next_child = container.first_child();
        while let Some(child) = next_child {
            let size = children_sizes[i];
            let (child_min, child_nat, _, _) = child.measure(opposite_orientation, size);
            minimum = minimum.max(child_min);
            natural = natural.max(child_nat);

            next_child = child.next_sibling();
            i += 1;
        }

        (minimum, natural)
    }

    fn get_preferred_size_for_same_orientation(
        &self,
        paned: &Container,
        orientation: gtk4::Orientation,
        for_size: i32,
    ) -> (i32, i32) {
        // For a container like this [   |   ] (vertical split),
        // this means we are given height and need to calculate width

        let mut minimum = 0;
        let mut natural = 0;

        let mut next_child = paned.first_child();
        while let Some(child) = next_child {
            // let container: Container = parent.downcast();
            if let Ok(separator) = child.clone().downcast::<Bin>() {
                let (_, handle_size, _, _) = separator.measure(orientation, -1);
                minimum += handle_size;
                natural += handle_size;
            } else {
                let (child_min, child_nat, _, _) = child.measure(orientation, for_size);
                minimum += child_min;
                natural += child_nat;
            }

            next_child = child.next_sibling();
        }

        (minimum, natural)
    }

    #[inline]
    fn get_children_sizes(
        &self,
        container: &Container,
        size: i32,
    ) -> Vec<i32> {
        // Percentages might be floats, but sizes are integer pixels
        let child_count = container.children_count();
        let mut children_sizes = Vec::with_capacity((child_count * 2) - 1);

        let mut remaining_size = size;
        // Debt tracks half of previous handle size
        let mut handle_debt = 0;

        let mut next_child = container.first_child();
        while let Some(child) = next_child {
            if let Some(separator) = child.next_sibling() {
                let separator: Separator = separator.downcast().unwrap();
                if size > -1 {
                    // We stil have a sibling, we take half of the separator size
                    let percentage = separator.get_percentage();

                    // Get handle size first
                    let handle_size = separator.get_handle_width();
                    let half_handle = (handle_size as f64 * 0.5).round() as i32;

                    // Take handle size into accout (each child owns half of it), both previous
                    // (handle_debt) and current (half_handle)
                    let percentaged_size = (size as f64 * percentage).round() as i32;
                    let child_size = percentaged_size - handle_debt - half_handle;
                    children_sizes.push(child_size);
                    children_sizes.push(handle_size);

                    remaining_size -= percentaged_size;
                    handle_debt = handle_size - half_handle;
                } else {
                    children_sizes.push(size);
                    children_sizes.push(-1);
                }
                next_child = separator.next_sibling();
            } else {
                if size > -1 {
                    // No siblings left, we take all of the remaining size
                    children_sizes.push(remaining_size - handle_debt);
                } else {
                    children_sizes.push(size);
                }
                break;
            };
        }

        children_sizes
    }
}
