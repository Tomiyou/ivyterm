use std::rc::Rc;

use adw::{prelude::*, HeaderBar};
use adw::{Application, ApplicationWindow};
use gtk::{Box, ColorDialog, ColorDialogButton, Orientation};

pub const INITIAL_WIDTH: i32 = 800;
pub const INITIAL_HEIGHT: i32 = 640;
pub const APPLICATION_TITLE: &str = "Windbreeze";

pub struct GlobalSettings {
    app: Rc<Application>,
}

impl GlobalSettings {
    pub fn new(app: Rc<Application>) -> Self {
        Self { app }
    }

    pub fn show_settings_window(&self) {
        let window_box = Box::new(Orientation::Vertical, 0);

        let header_bar = HeaderBar::new();

        // Color picker
        let color_dialog = ColorDialog::new();
        let color_dialog_button = ColorDialogButton::new(Some(color_dialog));
        color_dialog_button.connect_rgba_notify(|button| {
            let rgba = button.rgba();
            println!("connect_rgba_notify executed {:?}", rgba);
        });

        window_box.append(&header_bar);
        window_box.append(&color_dialog_button);

        // Create a new window
        let window = ApplicationWindow::builder()
            .application(self.app.as_ref())
            .title(APPLICATION_TITLE)
            .default_width(INITIAL_WIDTH)
            .default_height(INITIAL_HEIGHT)
            .content(&window_box)
            .build();

        window.present();
    }
}
