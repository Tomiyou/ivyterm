use general::create_general_page;
use gtk4::{Align, Box, ColorDialog, ColorDialogButton, Label, Orientation};
use keybindings::create_keybinding_page;
use libadwaita::{prelude::*, PreferencesGroup, PreferencesRow, PreferencesWindow};
use tmux::create_tmux_page;

use crate::{application::IvyApplication, config::IvyColor};

mod general;
mod keybindings;
mod tmux;

fn create_setting_row(pref_group: &PreferencesGroup, name: &str, child: impl IsA<gtk4::Widget>) {
    child.set_halign(Align::End);

    let label = Label::builder()
        .hexpand(true)
        .halign(Align::Start)
        .label(name)
        .build();

    let row_box = Box::new(Orientation::Horizontal, 0);
    row_box.append(&label);
    row_box.append(&child);

    let row = PreferencesRow::builder()
        .title(name)
        .child(&row_box)
        .css_classes(["setting_row"])
        .build();

    pref_group.add(&row);
}

fn create_color_button(data: &IvyColor) -> ColorDialogButton {
    let button = ColorDialogButton::new(Some(ColorDialog::new()));
    button.set_rgba(data.as_ref());

    button
}

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

    // Tmux settings page
    let tmux_page = create_tmux_page(app, &data);
    window.add(&tmux_page);

    // Keybinding settings page
    let keybinding_page = create_keybinding_page(app);
    window.add(&keybinding_page);

    window.present();
}
