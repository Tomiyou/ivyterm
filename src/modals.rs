use gtk4::{Box, Button, Entry, Label, Orientation};
use libadwaita::{prelude::*, HeaderBar, Window};

use crate::application::IvyApplication;

pub fn spawn_rename_modal(
    parent: &libadwaita::ApplicationWindow,
    old_name: &str,
    callback: glib::RustClosure,
) {
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

pub fn spawn_new_tmux_modal(parent: &libadwaita::ApplicationWindow) {
    let app = parent.application().unwrap();

    let dialog = Window::builder()
        .application(&app)
        .title("Attach new Tmux session")
        // .default_height(200)
        // .default_width(400)
        .modal(true)
        .transient_for(parent)
        .build();

    let header_bar = HeaderBar::new();
    let content = Box::new(Orientation::Vertical, 5);

    // Tmux session input
    let session_label = Label::new(Some("Tmux session:"));
    let session_input = Entry::new();
    content.append(&session_label);
    content.append(&session_input);

    // SSH input
    let ssh_label = Label::new(Some("SSH host:"));
    let ssh_input = Entry::new();
    content.append(&ssh_label);
    content.append(&ssh_input);

    // Button
    let button = Button::builder().label("Attach").build();
    content.append(&button);

    let window_box = Box::new(Orientation::Vertical, 0);
    window_box.append(&header_bar);
    window_box.append(&content);
    dialog.set_content(Some(&window_box));

    button.connect_clicked(glib::clone!(
        #[weak]
        dialog,
        move |_| {
            let tmux_session = session_input.text();
            let ssh_target = ssh_input.text();

            let app = dialog.application();
            dialog.close();

            if let Some(app) = app {
                let app: IvyApplication = app.downcast().unwrap();
                let ssh_target = if ssh_target.is_empty() {
                    None
                } else {
                    Some(ssh_target.as_str())
                };
                app.new_window(Some(tmux_session.as_str()), ssh_target);
            }
        }
    ));

    dialog.present();
}
