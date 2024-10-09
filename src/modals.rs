use gtk4::{Box, Button, Entry, Orientation};
use libadwaita::{prelude::*, HeaderBar, Window};

pub fn spawn_rename_modal(parent: &libadwaita::ApplicationWindow, old_name: &str, callback: glib::RustClosure) {
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
    let content = Box::new(Orientation::Vertical, 0);
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
