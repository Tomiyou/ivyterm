#![allow(deprecated)]
use std::{cell::RefCell, rc::Rc};

use general::create_general_page;
use gtk4::{Align, Box, ColorButton, Label, Orientation};
use keybindings::KeybindingPage;
use libadwaita::{prelude::*, PreferencesGroup, PreferencesRow, PreferencesWindow};
use tmux::create_tmux_page;

use crate::{
    application::IvyApplication,
    config::{GlobalConfig, IvyColor},
};

mod general;
mod keybindings;
mod tmux;

pub fn spawn_preferences_window(app: &IvyApplication, config: GlobalConfig) {
    let config = Rc::new(RefCell::new(config));

    // Settings window doesn't exist yet, we need to build it now
    let window = PreferencesWindow::builder().application(app).build();

    // General settings page
    let general_page = create_general_page(&config);
    window.add(&general_page);

    // Tmux settings page
    let tmux_page = create_tmux_page(&config);
    window.add(&tmux_page);

    // Keybinding settings page
    let keybinding_page = KeybindingPage::new(app);
    window.add(&keybinding_page);

    // Connect window to update app config when it exits
    window.connect_destroy(glib::clone!(
        #[weak]
        app,
        #[weak]
        keybinding_page,
        move |_| {
            let mut config = config.borrow().clone();
            let keybindings = keybinding_page.get_keybindings();
            config.keybindings.update(&keybindings);
            // Copy updated config back to application
            app.update_config(config, keybindings);
        }
    ));

    window.present();
}

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

fn create_color_button(data: &IvyColor) -> ColorButton {
    let button = ColorButton::with_rgba(data.as_ref());

    button
}
