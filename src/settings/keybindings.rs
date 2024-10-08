use gtk4::{Align, Box, Label, Orientation};
use libadwaita::{prelude::*, PreferencesGroup, PreferencesPage, PreferencesRow};

use crate::{application::IvyApplication, keyboard::Keybinding};

fn create_keybinding_row(keybind: Keybinding) -> PreferencesRow {
    let row_box = Box::new(Orientation::Horizontal, 0);

    let label = Label::builder()
        .label(keybind.description)
        .halign(Align::Start)
        .hexpand(true)
        .build();
    row_box.append(&label);

    let keybind = Label::builder()
        .label(keybind.text)
        .halign(Align::End)
        .build();
    row_box.append(&keybind);

    let row = PreferencesRow::builder()
        .child(&row_box)
        .css_classes(["setting_row"])
        .build();

    row
}

pub fn create_keybinding_page(app: &IvyApplication) -> PreferencesPage {
    let group = PreferencesGroup::new();

    let keybindings = app.get_keybindings();
    for keybind in keybindings {
        let row = create_keybinding_row(keybind);
        group.add(&row);
    }

    let page = PreferencesPage::builder().title("Keybindings").build();
    page.add(&group);
    page
}
