use std::cell::Cell;

use gtk4::{LayoutManager, Orientation, Widget};
use libadwaita::glib;
use libadwaita::subclass::prelude::*;
use vte4::{Cast, LayoutManagerExt, WidgetExt};

use crate::toplevel::TopLevel;

// Object holding the state
pub struct TopLevelLayoutPriv {
    last_allocated_size: Cell<(i32, i32)>,
}

impl Default for TopLevelLayoutPriv {
    fn default() -> Self {
        Self {
            last_allocated_size: Cell::new((0, 0)),
        }
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for TopLevelLayoutPriv {
    const NAME: &'static str = "IvyTerminalTopLevelLayout";
    type Type = super::TopLevelLayout;
    type ParentType = LayoutManager;
}

// Trait shared by all GObjects
impl ObjectImpl for TopLevelLayoutPriv {}

impl LayoutManagerImpl for TopLevelLayoutPriv {
    fn measure(
        &self,
        widget: &Widget,
        orientation: Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        let mut min = 0;
        let mut nat = 0;
        let mut min_baseline = -1;
        let mut nat_baseline = -1;

        let mut next_child = widget.first_child();
        while let Some(child) = next_child {
            if child.should_layout() {
                let (child_min, child_nat, child_min_baseline, child_nat_baseline) =
                    child.measure(orientation, for_size);
                min = min.max(child_min);
                nat = nat.max(child_nat);
                if child_min_baseline > -1 {
                    min_baseline = min_baseline.max(child_min_baseline);
                }
                if child_nat_baseline > -1 {
                    nat_baseline = nat_baseline.max(child_nat_baseline);
                }
            }

            next_child = child.next_sibling();
        }

        (min, nat, min_baseline, nat_baseline)
    }

    fn allocate(&self, widget: &Widget, width: i32, height: i32, baseline: i32) {
        let mut next_child = widget.first_child();
        while let Some(child) = next_child {
            if child.should_layout() {
                child.allocate(width, height, baseline, None);
            }

            next_child = child.next_sibling();
        }

        let last_allocated_size = self.last_allocated_size.get();
        let new_allocated_size = (width, height);
        if last_allocated_size != new_allocated_size {
            self.last_allocated_size.replace(new_allocated_size);

            if let Some(top_level) = self.obj().widget() {
                let top_level: TopLevel = top_level.downcast().unwrap();
                top_level.layout_alloc_changed();
            }
        }
    }
}
