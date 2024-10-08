use gtk4::{Orientation, Paned, Widget};
use libadwaita::prelude::*;
use vte4::Terminal;

use crate::mux::terminal::create_terminal;

use super::toplevel::TopLevel;

pub fn new_paned(
    orientation: Orientation,
    start_child: impl IsA<Widget>,
    end_child: impl IsA<Widget>,
) -> Paned {
    let paned = Paned::builder()
        .focusable(true)
        .vexpand(true)
        .hexpand(true)
        .wide_handle(true)
        .css_classes(vec!["terminal-pane"])
        .orientation(orientation)
        .start_child(&start_child)
        .end_child(&end_child)
        .build();

    paned
}

pub fn split_pane(paned: Paned, orientation: Orientation, top_level: &TopLevel) {
    let start_child = paned.start_child().unwrap();
    if start_child.has_focus() {
        // Replace first child
        paned.set_start_child(None::<&Widget>);

        let terminal = create_terminal(top_level);
        let new_paned = new_paned(orientation, start_child, terminal);
        paned.set_start_child(Some(&new_paned));

        // Re-calculate panes divider position
        let size = paned.size(paned.orientation());
        paned.set_position(size / 2);
        return;
    }

    let end_child = paned.end_child().unwrap();
    if end_child.has_focus() {
        // Replace end child
        paned.set_end_child(None::<&Widget>);

        let terminal = create_terminal(top_level);
        let new_paned = new_paned(orientation, end_child, terminal);
        paned.set_end_child(Some(&new_paned));

        // Re-calculate panes divider position
        let size = paned.size(paned.orientation());
        paned.set_position(size / 2);
        return;
    }
}

// TODO: Move all of this into top_level, since it can check top_level directly using pointers
pub fn close_pane(closing_paned: Paned, closing_terminal: Terminal) {
    // Paned always has 2 children present, if not, then it would have been deleted
    let start_child = closing_paned.start_child().unwrap();
    let end_child = closing_paned.end_child().unwrap();

    let retained_child = if start_child == closing_terminal {
        // Remove start child, keep last child
        end_child
    } else if end_child == closing_terminal {
        // Remove last child, keep first child
        start_child
    } else {
        panic!("Trying to close pane, but none of the children is the closed terminal");
    };

    closing_paned.set_start_child(None::<&Widget>);
    closing_paned.set_end_child(None::<&Widget>);

    // Determine if parent is type Bin or Paned
    let parent = closing_paned.parent().unwrap();

    if let Ok(parent) = parent.clone().downcast::<TopLevel>() {
        // Parent is TopLevel
        parent.set_child(Some(&retained_child));
        retained_child.grab_focus();
        return;
    }

    if let Ok(parent) = parent.downcast::<Paned>() {
        // Parent is another gtk4::Paned
        parent.emit_cycle_child_focus(true);

        let start_child = parent.start_child().unwrap();
        let end_child = parent.end_child().unwrap();

        if start_child == closing_paned {
            // Closing Pane is start child
            parent.set_start_child(Some(&retained_child));
        } else if end_child == closing_paned {
            // Closing Pane is end child
            parent.set_end_child(Some(&retained_child));
        } else {
            panic!("Closing Pane is neither first nor end child");
        }

        // Re-adjust split positions
        let size = parent.size(parent.orientation());
        parent.set_position(size / 2);

        // TODO: We need for another terminal to grab focus
        parent.emit_cycle_child_focus(true);
        return;
    }

    panic!("Parent is neither Bin nor Paned");
}
