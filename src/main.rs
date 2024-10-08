use application::IvyApplication;
use libadwaita::glib;
use libadwaita::prelude::*;

mod application;
mod helpers;
mod keyboard;
mod normal_widgets;
mod settings;
mod tmux_api;
mod tmux_widgets;

fn main() -> glib::ExitCode {
    let application = IvyApplication::new();

    // Initialize IvyApplication
    application.connect_startup(|app| {
        app.init_css_provider();
        app.init_keybindings();
    });

    application.connect_activate(move |app| {
        // app.new_window(None);
        app.new_window(Some("blabla"));
    });
    application.run()
}
