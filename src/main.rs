use std::sync::atomic::AtomicU32;

use gtk4::gdk::Display;
use gtk4::CssProvider;
use libadwaita::prelude::*;
use libadwaita::Application;

use global_state::{APPLICATION_TITLE, INITIAL_HEIGHT, INITIAL_WIDTH};
use tmux::attach_tmux;
use window::IvyWindow;

mod error;
mod global_state;
mod keyboard;
mod pane;
mod separator;
mod tmux;
mod toplevel;
mod window;

static GLOBAL_TAB_ID: AtomicU32 = AtomicU32::new(0);

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("style.css"));

    // Add the provider to the default screen
    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn create_window(app: &Application, tmux_session: Option<&str>) {
    // Create a new window
    let window = IvyWindow::new(app, APPLICATION_TITLE, INITIAL_WIDTH, INITIAL_HEIGHT);

    if let Some(session_name) = tmux_session {
        println!("Starting TMUX");
        let tmux = attach_tmux(session_name, &window).unwrap();
        window.init_tmux(tmux);
    } else {
        // Create initial Tab
        window.new_tab(None);
    }

    window.present();
}

fn main() -> glib::ExitCode {
    let application = Application::builder()
        .application_id("com.tomiyou.ivyTerm")
        .build();

    application.connect_startup(|_| load_css());
    application.connect_activate(|app| {
        create_window(app, None);
        // create_window(app, Some("terminator"));
    });
    application.run()
}
