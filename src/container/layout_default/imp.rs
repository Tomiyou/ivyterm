use std::cell::RefCell;

use gtk4::{Allocation, LayoutManager, Orientation};
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, Bin};
use vte4::{Cast, WidgetExt};

use crate::container::separator::Separator;
use crate::container::Container;

// Object holding the state
#[derive(Default)]
pub struct ContainerLayoutPriv {
    pub separators: RefCell<Vec<Separator>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ContainerLayoutPriv {
    const NAME: &'static str = "IvyTerminalContainerLayout";
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
            children_sizes
                .iter()
                .map(|child_width| {
                    let allocation = Allocation::new(x, 0, *child_width, height);
                    x += child_width;

                    allocation
                })
                .collect()
        } else {
            let children_sizes = self.get_children_sizes(&paned, height);

            let mut y = 0;
            children_sizes
                .iter()
                .map(|child_height| {
                    let allocation = Allocation::new(0, y, width, *child_height);
                    y += child_height;

                    allocation
                })
                .collect()
        };

        let mut children_iter = paned.first_child();
        let mut allocation_iter = allocations.iter();
        while let Some(child) = children_iter {
            let allocation = allocation_iter.next().unwrap();
            child.size_allocate(&allocation, -1);
            children_iter = child.next_sibling();
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
    fn get_children_sizes(&self, container: &Container, given_size: i32) -> Vec<i32> {
        let separators = self.separators.borrow();
        let separator_count = separators.len();
        let child_count = separator_count * 2 + 1;
        let mut children_sizes = Vec::with_capacity(child_count);

        // Handle being given size less than 0 (usually when not initialized yet or error)
        if given_size < 0 {
            for _ in 0..child_count {
                children_sizes.push(-1);
            }
            return children_sizes;
        }

        // We can assume that all Separators have the same handle width
        let handle_size = match separators.first() {
            Some(separator) => separator.get_handle_width(),
            None => 0
        };
        // From this point on, sizes of all Separator's handles have been removed from given_size
        let mut remaining_size = given_size;
        let given_size = given_size - (handle_size * separator_count as i32);
        let given_size = given_size as f64;

        let mut next_child = container.first_child();
        while let Some(child) = next_child {
            if let Some(separator) = child.next_sibling() {
                let separator: Separator = separator.downcast().unwrap();
                let percentage = separator.get_percentage();

                if handle_size != separator.get_handle_width() {
                    eprintln!("Separator's have different handle sizes!");
                }

                let child_size = (given_size * percentage).round() as i32;
                children_sizes.push(child_size);
                children_sizes.push(handle_size);

                remaining_size -= child_size + handle_size;
                next_child = separator.next_sibling();
            } else {
                // No siblings left, we take all of the remaining size
                children_sizes.push(remaining_size);
                break;
            };
        }

        children_sizes
    }
}
