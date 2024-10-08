use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use application::IvyApplication;
use libadwaita::glib;
use libadwaita::prelude::*;

mod application;
mod container;
mod error;
mod global_state;
mod keyboard;
mod terminal;
mod tmux;
mod toplevel;
mod window;

static GLOBAL_TAB_ID: AtomicU32 = AtomicU32::new(0);
pub fn next_unique_tab_id() -> u32 {
    GLOBAL_TAB_ID.fetch_add(1, Ordering::Relaxed)
}

static GLOBAL_PANE_ID: AtomicU32 = AtomicU32::new(0);
pub fn next_unique_pane_id() -> u32 {
    GLOBAL_PANE_ID.fetch_add(1, Ordering::Relaxed)
}

fn main() -> glib::ExitCode {
    let application = IvyApplication::new();

    // Initialize CSS and load provider for later use
    application.connect_startup(|app| {
        app.init_css_provider();
    });

    application.connect_activate(move |app| {
        app.new_window(None);
        // create_window(app, Some("blabla"));
    });
    application.run()
}
