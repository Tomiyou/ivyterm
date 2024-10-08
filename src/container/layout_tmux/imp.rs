use std::cell::{Cell, RefCell};

use gtk4::{Allocation, LayoutManager, Orientation, Widget};
use libadwaita::subclass::prelude::*;
use libadwaita::{glib, Bin};
use vte4::{Cast, WidgetExt};

use crate::container::separator::Separator;
use crate::container::Container;

pub struct TmuxSeparator {
    pub s: Separator,
    pub position: i32,
}

// Object holding the state
#[derive(Default)]
pub struct TmuxLayoutPriv {
    pub separators: RefCell<Vec<TmuxSeparator>>,
    pub char_size: Cell<(i32, i32)>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TmuxLayoutPriv {
    const NAME: &'static str = "IvyTerminalTmuxLayout";
    type Type = super::TmuxLayout;
    type ParentType = LayoutManager;
}

// Trait shared by all GObjects
impl ObjectImpl for TmuxLayoutPriv {}

// How Tmux layout manger works: All the terminals inside this Window MUST have the same
// character width and height. All the spacing between any widgets inside TopLevel MUST
// be 0. Separator widget has the same handle width as Terminal's character is tall/wide
// (-2 since VTE widget has fixed internal padding of 1px on each side).
// Assuming all of the above holds: Tmux client size is simply calculated:
// -- cols: ((width - 2px) / char_width).floor() => this calculates how many chars fit in
//          a single line (2px accounting for internal VTE padding). Rows are calculated
//          the same way.
// Layout is given a position of each Separator in rows/cols and UNLESS Container is
// RESIZED, calculation will ALWAYS use rows/cols instead of percentages. If the Container
// is resized, percentages are adjusted FIRST and sizes are derived, then a size sync
// with Tmux is initiated if needed.
impl LayoutManagerImpl for TmuxLayoutPriv {
    fn measure(
        &self,
        widget: &Widget,
        orientation: Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        let container: Container = widget.clone().downcast().unwrap();

        let (minimum, natural) = if orientation == container.orientation() {
            self.get_preferred_size_for_same_orientation(&container, orientation, for_size)
        } else {
            self.get_preferred_size_for_opposite_orientation(&container, orientation, for_size)
        };

        (minimum, natural, -1, -1)
    }

    fn allocate(&self, widget: &Widget, width: i32, height: i32, _baseline: i32) {
        println!("Allocate {} x {}", width, height);
        let container: Container = widget.clone().downcast().unwrap();
        let orientation = container.orientation();

        let allocations: Vec<Allocation> = if orientation == Orientation::Horizontal {
            let children_sizes = self.get_children_sizes(&container, width);

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
            let children_sizes = self.get_children_sizes(&container, height);

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

        let mut children_iter = container.first_child();
        let mut allocation_iter = allocations.iter();
        while let Some(child) = children_iter {
            let allocation = allocation_iter.next().unwrap();
            child.size_allocate(&allocation, -1);
            children_iter = child.next_sibling();
        }
    }
}

impl TmuxLayoutPriv {
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
    fn get_children_sizes(&self, container: &Container, size: i32) -> Vec<i32> {
        let separators = self.separators.borrow();
        let child_count = (separators.len() * 2) + 1;
        let mut children_sizes = Vec::with_capacity(child_count);

        // Handle being given size less than 0 (usually when not initialized yet or error)
        if size < 0 {
            for _ in 0..child_count {
                children_sizes.push(-1);
            }
            return children_sizes;
        }

        // All Separators are the same size
        let handle_size = separators.first().unwrap().s.get_handle_width();
        // Cell size is 2px larger than handle_size, since we must account for VTE widget
        // fixed padding of 1px on each side
        let cell_size = handle_size + 2;
        let mut already_used_size = 0;

        for separator in separators.iter() {
            // Each child size is calculated like this: position of the Separator
            // (position in cells * cell_size) + 2 (accounting for VTE widget padding)
            //  We then subtract how much size we used up to this point
            let separator_position = (separator.position * cell_size) + 2;
            let child_size = separator_position - already_used_size;
            children_sizes.push(child_size);
            children_sizes.push(handle_size);

            already_used_size += child_size + handle_size;
        }
        children_sizes.push(size - already_used_size);

        children_sizes
    }
}
