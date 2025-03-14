use application::IvyApplication;
use libadwaita::glib;
use libadwaita::prelude::*;

mod application;
mod config;
mod helpers;
mod keyboard;
mod modals;
mod normal_widgets;
mod settings_window;
mod ssh;
mod tmux_api;
mod tmux_widgets;

fn main() -> glib::ExitCode {
    env_logger::init();

    let application = IvyApplication::new();

    // Initialize IvyApplication
    application.connect_startup(|app| {
        app.init_css_provider();
        app.init_keybindings();
    });

    application.connect_activate(move |app| {
        app.new_normal_window();
    });
    application.run()
}
