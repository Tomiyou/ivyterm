use application::IvyApplication;
use libadwaita::glib;
use libadwaita::prelude::*;

mod application;
mod container;
mod error;
mod keyboard;
mod settings;
mod terminal;
mod tmux;
mod toplevel;
mod window;

fn main() -> glib::ExitCode {
    let application = IvyApplication::new();

    // Initialize IvyApplication
    application.connect_startup(|app| {
        app.init_css_provider();
        app.init_keybindings();
    });

    application.connect_activate(move |app| {
        app.new_window(None);
        // create_window(app, Some("blabla"));
    });
    application.run()
}
