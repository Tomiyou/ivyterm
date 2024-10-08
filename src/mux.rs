use std::sync::atomic::Ordering;

use gtk4::{Orientation, Paned, Widget};
use libadwaita::{prelude::*, TabView};

use crate::{toplevel::TopLevel, GLOBAL_TAB_ID};

pub fn create_tab(tab_view: &TabView) {
    let tab_id = GLOBAL_TAB_ID.fetch_add(1, Ordering::Relaxed);
    let top_level = TopLevel::new(tab_view);

    // Add pane as a page
    let page = tab_view.append(&top_level);

    let text = format!("Terminal {}", tab_id);
    page.set_title(&text);
    tab_view.set_selected_page(&page);
}

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
