use general::create_general_page;
use keybindings::create_keybinding_page;
use libadwaita::{prelude::*, PreferencesWindow};

use crate::application::IvyApplication;

mod general;
mod keybindings;
mod tmux;

pub fn show_preferences_window(app: &IvyApplication) {
    // If a Settings window is already open, simply bring it to the front
    for window in app.windows() {
        if let Ok(window) = window.downcast::<PreferencesWindow>() {
            println!("Presenting an already open Settings window");
            window.present();
            return;
        }
    }

    // Settings window doesn't exist yet, we need to build it now
    let window = PreferencesWindow::builder().application(app).build();
    let data = app.get_full_config();

    // General settings page
    let general_page = create_general_page(app, &data);
    window.add(&general_page);

    // // Tmux settings page
    // let tmux_page = create_tmux_page(app, &data);
    // window.add(&tmux_page);

    // Keybinding settings page
    let keybinding_page = create_keybinding_page(app);
    window.add(&keybinding_page);

    window.present();
}
