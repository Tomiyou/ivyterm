use glib::RustClosure;
use gtk4::{Align, Box, Button, Entry, Label, Orientation};
use libadwaita::{prelude::*, ApplicationWindow, HeaderBar, Window};

pub fn spawn_rename_modal(parent: &ApplicationWindow, old_name: &str, callback: RustClosure) {
    let app = parent.application().unwrap();

    let dialog = Window::builder()
        .application(&app)
        .title("Rename tab...")
        // .default_height(200)
        // .default_width(400)
        .modal(true)
        .transient_for(parent)
        .build();

    let header_bar = HeaderBar::new();
    let content = Box::new(Orientation::Vertical, 5);
    let name_input = Entry::builder().placeholder_text(old_name).build();
    content.append(&name_input);
    let button = Button::builder().label("Rename").build();
    content.append(&button);

    let window_box = Box::new(Orientation::Vertical, 0);
    window_box.append(&header_bar);
    window_box.append(&content);
    dialog.set_content(Some(&window_box));

    // Close Dialog when user renames Tab
    button.connect_clicked(glib::clone!(
        #[weak]
        dialog,
        move |_| {
            let new_name = name_input.text();
            callback.invoke::<()>(&[&new_name.as_str()]);
            dialog.close();
        }
    ));

    dialog.present();
}

pub fn spawn_exit_modal(parent: &ApplicationWindow, confirm_callback: RustClosure) -> Window {
    let app = parent.application().unwrap();

    let dialog = Window::builder()
        .application(&app)
        .title("Close?")
        .modal(true)
        .transient_for(parent)
        .build();

    let window_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .hexpand(true)
        .vexpand(true)
        .build();

    // Window title bar
    let title = Label::new(Some("Close?"));
    let header_bar = HeaderBar::builder().title_widget(&title).build();
    window_box.append(&header_bar);

    // Content box
    let content = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();

    // Buttons
    let cancel = Button::builder().label("Cancel").build();
    cancel.connect_clicked(glib::clone!(
        #[weak]
        dialog,
        move |_| {
            dialog.close();
        }
    ));
    let confirm = Button::builder().label("Close Terminals").build();
    confirm.connect_clicked(glib::clone!(
        #[weak]
        parent,
        #[weak]
        dialog,
        move |_| {
            confirm_callback.invoke::<()>(&[]);
            dialog.close();
            parent.close();
        }
    ));

    let buttons = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(5)
        .halign(Align::Center)
        .build();
    buttons.append(&cancel);
    buttons.append(&confirm);

    // Labels
    let heading = Label::builder()
        .label("Close multiple terminals?")
        .css_classes(["close_confirm_heading"])
        .build();
    let message = Label::builder()
        .label("This window has several terminals open. Closing the window will\n also close all terminals within it.")
        .justify(gtk4::Justification::Center)
        .margin_start(20)
        .margin_end(20)
        .build();
    content.append(&heading);
    content.append(&message);
    content.append(&buttons);

    window_box.append(&content);
    dialog.set_content(Some(&window_box));
    dialog.present();

    dialog
}
