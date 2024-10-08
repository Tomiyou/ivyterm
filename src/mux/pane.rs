use gtk4::{Orientation, Paned, Widget};
use libadwaita::prelude::*;

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
