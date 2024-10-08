use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::{
    gdk::RGBA, pango::FontDescription, Box, ColorDialog, ColorDialogButton, FontDialog,
    FontDialogButton, Orientation,
};
use libadwaita::{prelude::*, ApplicationWindow, HeaderBar};

use super::IvyApplication;

pub const INITIAL_WIDTH: i32 = 802;
pub const INITIAL_HEIGHT: i32 = 648;
pub const APPLICATION_TITLE: &str = "ivyTerm";
pub const SPLIT_HANDLE_WIDTH: i32 = 9;

pub struct GlobalConfig {
    pub font_desc: FontDescription,
    pub main_colors: [RGBA; 2],
    pub palette_colors: [RGBA; 16],
    pub scrollback_lines: u32,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        // TODO: Parse config from file

        // Font
        let font_desc = FontDescription::from_string("CommitMono weight=400 13");

        // Colors
        let foreground = RGBA::new(1.0, 1.0, 1.0, 1.0);
        let background = RGBA::new(0.0, 0.0, 0.0, 1.0);
        let ambience_colors = [
            // Standard colors
            RGBA::parse("#2e3436").unwrap(),
            RGBA::parse("#cc0000").unwrap(),
            RGBA::parse("#4e9a06").unwrap(),
            RGBA::parse("#c4a000").unwrap(),
            RGBA::parse("#3465a4").unwrap(),
            RGBA::parse("#75507b").unwrap(),
            RGBA::parse("#06989a").unwrap(),
            RGBA::parse("#d3d7cf").unwrap(),
            // Bright colors
            RGBA::parse("#555753").unwrap(),
            RGBA::parse("#ef2929").unwrap(),
            RGBA::parse("#8ae234").unwrap(),
            RGBA::parse("#fce94f").unwrap(),
            RGBA::parse("#729fcf").unwrap(),
            RGBA::parse("#ad7fa8").unwrap(),
            RGBA::parse("#34e2e2").unwrap(),
            RGBA::parse("#eeeeec").unwrap(),
        ];

        Self {
            // font_desc: FontDescription::default(),
            font_desc,
            main_colors: [foreground, background],
            palette_colors: ambience_colors,
            scrollback_lines: 2000,
        }
    }
}

impl IvyApplication {
    pub fn get_terminal_config(&self) -> (FontDescription, [RGBA; 2], [RGBA; 16], u32) {
        let global_config = self.imp().config.borrow();
        let font_desc = global_config.font_desc.clone();
        let main_colors = global_config.main_colors.clone();
        let palette_colors = global_config.palette_colors.clone();
        let scrollback_lines = global_config.scrollback_lines.clone();

        (font_desc, main_colors, palette_colors, scrollback_lines)
    }

    pub fn show_settings_window(&self) {
        let window_box = Box::new(Orientation::Vertical, 0);

        // Window handle and buttons
        let header_bar = HeaderBar::new();

        // Font picker
        let font_dialog = FontDialog::new();
        let font_dialog_button = FontDialogButton::new(Some(font_dialog));
        font_dialog_button.connect_font_desc_notify(|button| {
            let font_description = button.font_desc().unwrap();
            println!(
                "connect_font_desc_notify executed {:?}",
                font_description.to_string()
            );
        });

        // Color picker
        let color_dialog = ColorDialog::new();
        let color_dialog_button = ColorDialogButton::new(Some(color_dialog));

        window_box.append(&header_bar);
        window_box.append(&font_dialog_button);
        window_box.append(&color_dialog_button);

        // Create a new window
        let window = ApplicationWindow::builder()
            .application(self)
            .title(APPLICATION_TITLE)
            .content(&window_box)
            .build();

        window.present();

        color_dialog_button.connect_rgba_notify(move |button| {
            let rgba = button.rgba();
            println!("connect_rgba_notify executed {:?}", rgba);
            let app = window.application();
            if let Some(app) = app {
                let app: IvyApplication = app.downcast().unwrap();
                app.change_background_color(rgba);
            }
        });
    }
}
