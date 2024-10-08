use std::cell::Cell;

use gtk4::{LayoutManager, Orientation};
use libadwaita::glib;
use libadwaita::subclass::prelude::*;
use vte4::{Cast, ObjectType, OrientableExt, WidgetExt};

use crate::paned::IvyPaned;

// Object holding the state
pub struct IvyLayoutPriv {
    pub percentage: Cell<f32>,
}

impl Default for IvyLayoutPriv {
    fn default() -> Self {
        Self {
            percentage: Cell::new(0.5),
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

        println!(
            "measure called for widget {:p}, orientation {:?} and size {}",
            paned.as_ptr(),
            orientation,
            for_size
        );
        if orientation == paned.orientation() {
            get_preferred_size_for_orientation(&paned, orientation, for_size)
        } else {
            get_preferred_size_for_opposite_orientation(&paned, orientation, for_size)
        }
    }

    fn allocate(&self, widget: &gtk4::Widget, width: i32, height: i32, baseline: i32) {
        println!("allocate widget {:p}", widget);
        self.parent_allocate(widget, width, height, baseline)
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

fn compute_position(
    paned: &IvyPaned,
    allocation: i32,
    start_child_req: i32,
    end_child_req: i32,
) -> (i32, i32, i32) {
    let min = 0;
    let max = allocation.max(min);

    let start_child_req = start_child_req as f32;
    let end_child_req = end_child_req as f32;
    let allocation = allocation as f32;

    // let pos: i32 = if !paned.position_set() {
    let pos = if start_child_req + end_child_req != 0.0 {
        allocation * (start_child_req / (start_child_req + end_child_req)) + 0.5
        // allocation * ((double)start_child_req / (start_child_req + end_child_req)) + 0.5
    } else {
        allocation * 0.5 + 0.5
    };
    // } else {
    //     if paned.last_allocation() > 0 {
    //         allocation * ((double) paned->start_child_size / (paned->last_allocation)) + 0.5
    //     } else {
    //         // OK
    //         paned->start_child_size
    //     }
    // };

    let pos = pos as i32;
    let pos = pos.clamp(min, max);

    (min, max, pos)
}

#[inline]
fn get_opposite_orientation(orientation: Orientation) -> Orientation {
    match orientation {
        Orientation::Horizontal => Orientation::Vertical,
        Orientation::Vertical => Orientation::Horizontal,
        _ => panic!("What the fuck is this orientation"),
    }
}

fn get_preferred_size_for_opposite_orientation(
    paned: &IvyPaned,
    opposite_orientation: gtk4::Orientation,
    size: i32,
) -> (i32, i32, i32, i32) {
    let (separator, _) = paned.get_separator();
    let start_child = paned.start_child();
    let end_child = paned.end_child();
    let mut separator_visible = false;
    let orientation = get_opposite_orientation(opposite_orientation);

    // `Size` is the size of the widget in the orientation of the Paned widget
    // We can calculate the position correctly now
    let (for_start_child, for_end_child, for_handle) = if size > -1 {
        let start_child = start_child.as_ref().unwrap();
        let end_child = end_child.as_ref().unwrap();

        if start_child.is_visible() && end_child.is_visible() {
            separator_visible = true;

            let (_, handle_width, _, _) = separator.measure(orientation, -1);
            let (start_child_min, _, _, _) = start_child.measure(orientation, -1);
            let (end_child_min, _, _, _) = end_child.measure(orientation, -1);

            let (_, _, start_child_size) =
                compute_position(paned, size - handle_width, start_child_min, end_child_min);
            let end_child_size = size - start_child_size - handle_width;

            (start_child_size, end_child_size, handle_width)
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
            println!(
                "\tget_preferred_size_for_opposite_orientation: start_child {:?} {} {} {}",
                opposite_orientation, for_start_child, minimum, natural
            );
        }
    }

    if let Some(end_child) = end_child {
        if end_child.is_visible() {
            let (child_min, child_nat, _, _) =
                end_child.measure(opposite_orientation, for_end_child);
            minimum = minimum.max(child_min);
            natural = natural.max(child_nat);
            println!(
                "\tget_preferred_size_for_opposite_orientation: end_child {:?} {} {} {}",
                opposite_orientation, for_end_child, minimum, natural
            );
        }
    }

    if separator_visible {
        let (child_min, child_nat, _, _) = separator.measure(opposite_orientation, for_handle);
        minimum = minimum.max(child_min);
        natural = natural.max(child_nat);
        println!(
            "\tget_preferred_size_for_opposite_orientation: separator {:?} {} {} {}",
            opposite_orientation, for_handle, minimum, natural
        );
    }

    println!("\tget_preferred_size_for_opposite_orientation: returning {} {}", minimum, natural);
    (minimum, natural, -1, -1)
}

fn get_preferred_size_for_orientation(
    paned: &IvyPaned,
    orientation: gtk4::Orientation,
    for_size: i32,
) -> (i32, i32, i32, i32) {
    let mut minimum = 0;
    let mut natural = 0;

    println!("\tget_preferred_size_for_orientation {:?}, size {}", orientation, for_size);

    let start_child = paned.start_child();
    let end_child = paned.end_child();
    let mut separator_visible = false;

    if let Some(start_child) = start_child {
        if start_child.is_visible() {
            let (child_min, child_nat, _, _) = start_child.measure(orientation, for_size);
            minimum = child_min;
            natural = child_nat;
            separator_visible = true;
            println!(
                "\tget_preferred_size_for_orientation: start_child {} {}",
                child_min, child_nat
            );
        }
    }

    if let Some(end_child) = end_child {
        if end_child.is_visible() {
            let (child_min, child_nat, _, _) = end_child.measure(orientation, for_size);
            minimum += child_min;
            natural += child_nat;
            separator_visible = separator_visible && true;
            println!(
                "\tget_preferred_size_for_orientation: end_child {} {}",
                child_min, child_nat
            );
        }
    }

    let (separator, _) = paned.get_separator();
    if separator_visible {
        let (_, handle_size, _, _) = separator.measure(orientation, -1);
        minimum += handle_size;
        natural += handle_size;
        println!(
            "\tget_preferred_size_for_orientation: separator {}",
            handle_size
        );
    }

    println!("\tget_preferred_size_for_orientation: returning {} {}", minimum, natural);
    (minimum, natural, -1, -1)
}
