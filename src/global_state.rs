use std::sync::RwLock;

use gtk4::pango::FontDescription;
use lazy_static::lazy_static;
use libadwaita::{prelude::*, HeaderBar};
use libadwaita::{Application, ApplicationWindow};
use gtk4::{Box, ColorDialog, ColorDialogButton, FontDialog, FontDialogButton, Orientation};

pub const INITIAL_WIDTH: i32 = 802;
pub const INITIAL_HEIGHT: i32 = 648;
pub const APPLICATION_TITLE: &str = "ivyTerm";
pub const SPLIT_HANDLE_WIDTH: i32 = 1;

pub struct GlobalSettings {
    pub font_desc: FontDescription,
}

lazy_static! {
    pub static ref GLOBAL_SETTINGS: RwLock<GlobalSettings> = RwLock::new(GlobalSettings::new());
}

impl GlobalSettings {
    pub fn new() -> Self {
        let font_desc = FontDescription::from_string("CommitMono-Tomo weight=475 13");
        Self {
            // font_desc: FontDescription::default(),
            font_desc,
        }
    }
}

pub fn show_settings_window(app: Application) {
    let window_box = Box::new(Orientation::Vertical, 0);

    // Window handle and buttons
    let header_bar = HeaderBar::new();

    // Font picker
    let font_dialog = FontDialog::new();
    let font_dialog_button = FontDialogButton::new(Some(font_dialog));
    font_dialog_button.connect_font_desc_notify(|button| {
        let font_description = button.font_desc().unwrap();
        println!("connect_font_desc_notify executed {:?}", font_description.to_string());
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
        .application(&app)
        .title(APPLICATION_TITLE)
        .default_width(INITIAL_WIDTH)
        .default_height(INITIAL_HEIGHT)
        .content(&window_box)
        .build();

    window.present();
}
