use libadwaita::{prelude::*, HeaderBar};
use libadwaita::{Application, ApplicationWindow};
use gtk4::{Box, ColorDialog, ColorDialogButton, FontDialog, FontDialogButton, Orientation};

pub const INITIAL_WIDTH: i32 = 800;
pub const INITIAL_HEIGHT: i32 = 640;
pub const APPLICATION_TITLE: &str = "ivyTerm";

pub struct GlobalSettings {
    app: Application,
}

impl GlobalSettings {
    pub fn new(app: &Application) -> Self {
        Self {
            app: app.clone(),
        }
    }

    pub fn show_settings_window(&self) {
        let window_box = Box::new(Orientation::Vertical, 0);

        let header_bar = HeaderBar::new();

        // Font picker
        let font_dialog = FontDialog::new();
        let font_dialog_button = FontDialogButton::new(Some(font_dialog));
        font_dialog_button.connect_font_desc_notify(|button| {
            let font_description = button.font_desc();
            println!("connect_font_desc_notify executed {:?}", font_description);
        });

        // Color picker
        let color_dialog = ColorDialog::new();
        let color_dialog_button = ColorDialogButton::new(Some(color_dialog));
        color_dialog_button.connect_rgba_notify(|button| {
            let rgba = button.rgba();
            println!("connect_rgba_notify executed {:?}", rgba);
        });

        window_box.append(&header_bar);
        window_box.append(&font_dialog_button);
        window_box.append(&color_dialog_button);

        // Create a new window
        let window = ApplicationWindow::builder()
            .application(&self.app)
            .title(APPLICATION_TITLE)
            .default_width(INITIAL_WIDTH)
            .default_height(INITIAL_HEIGHT)
            .content(&window_box)
            .build();

        window.present();
    }
}
