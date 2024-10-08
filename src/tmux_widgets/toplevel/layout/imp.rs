use std::cell::Cell;

use gtk4::{LayoutManager, Orientation, Widget};
use libadwaita::glib;
use libadwaita::subclass::prelude::*;
use vte4::{Cast, LayoutManagerExt, WidgetExt};

use crate::tmux_widgets::toplevel::TmuxTopLevel;

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
    const NAME: &'static str = "ivytermTmuxTabPageLayout";
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
        let new_allocated_size = (width, height);
        let last_allocated_size = self.last_allocated_size.replace(new_allocated_size);

        // If size is different than previous cached size, we need to adjust Separator positions first,
        // so we don't get any negative sizes during allocation
        let mut top_level: Option<TmuxTopLevel> = None;
        if last_allocated_size != new_allocated_size {
            if let Some(_top_level) = self.obj().widget() {
                let _top_level: TmuxTopLevel = _top_level.downcast().unwrap();

                // Be careful we don't divide by 0
                let (x_diff, y_diff) = match last_allocated_size {
                    (0, 0) => (1f64, 1f64),
                    _ => {
                        let x_diff = new_allocated_size.0 as f64 / last_allocated_size.0 as f64;
                        let y_diff = new_allocated_size.1 as f64 / last_allocated_size.1 as f64;
                        (x_diff, y_diff)
                    }
                };
                // Go through entire hierarchy and adjust Separator positions
                _top_level.adjust_separator_positions(x_diff, y_diff);

                top_level = Some(_top_level);
            }
        }

        // Do the actual allocation()
        let mut next_child = widget.first_child();
        while let Some(child) = next_child {
            if child.should_layout() {
                child.allocate(width, height, baseline, None);
            }

            next_child = child.next_sibling();
        }

        // If size is different than previous cached size, we also have to resync Tmux session size,
        // but we can only do this here after allocation has already happened
        if let Some(top_level) = top_level {
            top_level.layout_alloc_changed();
        }
    }
}
