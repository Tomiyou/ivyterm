use libadwaita::glib::signal::Propagation;
use libadwaita::{prelude::*, Bin};
use gtk4::{EventControllerKey, Orientation, Paned, ScrolledWindow, Widget};

use crate::keyboard::{matches_keybinding, Keybinding};
use crate::mux::terminal::create_terminal;

pub struct Pane {}

impl Pane {
    pub fn new(
        orientation: Orientation,
        start_child: impl IsA<Widget>,
        end_child: impl IsA<Widget>,
    ) -> Paned {
        let paned = Paned::builder()
            .focusable(true)
            .vexpand(true)
            .hexpand(true)
            .css_classes(vec!["terminal-pane"])
            .orientation(orientation)
            .start_child(&start_child)
            .end_child(&end_child)
            .build();

        let eventctl = EventControllerKey::new();
        eventctl.connect_key_pressed(move |eventctl, keyval, keycode, state| {
            // Handle terminal splits
            let paned = eventctl.widget();
            let paned = paned.downcast::<Paned>().unwrap();

            // println!("Paned keycode {}", keycode);

            // Split vertical
            if matches_keybinding(keyval, keycode, state, Keybinding::PaneSplit(true)) {
                split_pane(paned, Orientation::Vertical);
                return Propagation::Stop;
            }
            // Split horizontal
            if matches_keybinding(keyval, keycode, state, Keybinding::PaneSplit(false)) {
                split_pane(paned, Orientation::Horizontal);
                return Propagation::Stop;
            }
            // Close pane
            if matches_keybinding(keyval, keycode, state, Keybinding::PaneClose) {
                println!("Closing pane");
                close_pane(paned);
                return Propagation::Stop;
            }

            Propagation::Proceed
        });
        paned.add_controller(eventctl);

        paned
    }
}

fn split_pane(paned: Paned, orientation: Orientation) {
    let none: Option<&Widget> = None;

    let start_child = paned.start_child().unwrap();
    if start_child.has_focus() {
        // Replace first child
        paned.set_start_child(none);

        let terminal = create_terminal();
        let new_paned = Pane::new(orientation, start_child, terminal);
        paned.set_start_child(Some(&new_paned));

        // Re-calculate panes divider position
        let size = paned.size(paned.orientation());
        paned.set_position(size / 2);
        println!("New PANE {:?}", new_paned.as_ptr());
        return;
    }

    let end_child = paned.end_child().unwrap();
    if end_child.has_focus() {
        // Replace end child
        paned.set_end_child(none);

        let terminal = create_terminal();
        let new_paned = Pane::new(orientation, end_child, terminal);
        paned.set_end_child(Some(&new_paned));

        // Re-calculate panes divider position
        let size = paned.size(paned.orientation());
        paned.set_position(size / 2);
        println!("New PANE {:?}", new_paned.as_ptr());
        return;
    }
}

pub fn close_pane(closing_paned: Paned) {
    let none: Option<&Widget> = None;

    // Paned always has 2 children present, if not, then it would have been deleted
    let start_child = closing_paned.start_child().unwrap();
    let end_child = closing_paned.end_child().unwrap();

    let retained_child = if start_child.has_focus() {
        // Remove start child, keep last child
        println!("Removing first child");
        end_child
    } else if end_child.has_focus() {
        // Remove last child, keep first child
        println!("Removing last child");
        start_child
    } else {
        panic!("Trying to close pane, but none of the children is focused");
    };

    closing_paned.set_start_child(none);
    closing_paned.set_end_child(none);

    // Determine if parent is type Bin or Paned
    let parent = closing_paned.parent().unwrap();
    println!(
        "Closing pane {:?}, parent {:?}",
        closing_paned.as_ptr(),
        parent.as_ptr()
    );

    if let Ok(parent) = parent.clone().downcast::<Bin>() {
        // Parent is libadwaita::Bin
        println!("Parent is libadwaita::Bin");
        parent.set_child(Some(&retained_child));
        return;
    }

    if let Ok(parent) = parent.downcast::<Paned>() {
        // Parent is another gtk4::Paned
        println!("Parent is gtk4::Paned");
        parent.emit_cycle_child_focus(true);

        // Check if closing_pane is start child
        let start_child = parent.start_child().unwrap();
        if closing_paned.eq(&start_child) {
            println!("Setting start child of parent");
            parent.set_start_child(Some(&retained_child));
    
            let size = parent.size(parent.orientation());
            parent.set_position(size / 2);
            return;
        }
    
        // Check if closing_pane is end child
        let end_child = parent.end_child().unwrap();
        if closing_paned.eq(&end_child) {
            println!("Setting end child of parent");
            parent.set_end_child(Some(&retained_child));
    
            let size = parent.size(parent.orientation());
            parent.set_position(size / 2);
            return;
        }
    }

    panic!("Parent is neither Bin nor Paned");
}