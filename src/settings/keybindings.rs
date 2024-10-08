// use gtk4::{Align, Box, ColumnView, ColumnViewColumn, Label, NoSelection, Orientation, SelectionModel, ShortcutsGroup, ShortcutsSection, ShortcutsShortcut};
// use libadwaita::{prelude::*, PreferencesGroup, PreferencesPage};

// use crate::application::IvyApplication;

// fn create_keybinding_row(description: &str, keybind) {
//     let row = Box::new(Orientation::Horizontal, 0);

//     let label = Label::builder()
//         .label(description)
//         .halign(Align::Start)
//         .build();

//     let keybind = Label::builder()
//         .label(keybind)
//         .halign(Align::End)
//         .build();
// }

// pub fn create_keybinding_page(app: &IvyApplication) -> PreferencesPage {
//     let shortcut1 = ShortcutsShortcut::builder().title("Lmao").accelerator("<Ctrl>F").build();
//     let shortcut2 = ShortcutsShortcut::builder().title("Lmao").accelerator("<Ctrl><Alt>K").build();

//     let group = ShortcutsGroup::builder().title("Group 1").build();
//     group.append(&shortcut1);
//     group.append(&shortcut2);
    
//     let section = ShortcutsSection::builder().title("Section 1").build();
//     section.append(&group);

//     let group = PreferencesGroup::new();
//     group.add(&section);

//     let page = PreferencesPage::builder().title("Keybindings").build();
//     page.add(&group);
//     page
// }
