mod pane;
mod terminal;
mod toplevel;

use std::sync::atomic::Ordering;

use libadwaita::TabView;

use crate::GLOBAL_TAB_ID;

use self::toplevel::TopLevel;

pub fn create_tab(tab_view: &TabView) {
    let tab_id = GLOBAL_TAB_ID.fetch_add(1, Ordering::Relaxed);
    let top_level = TopLevel::new(tab_view);

    // Add pane as a page
    let page = tab_view.append(&top_level);

    let text = format!("Terminal {}", tab_id);
    page.set_title(&text);
    tab_view.set_selected_page(&page);
}
