use general::create_general_page;
use gtk4::{gdk::RGBA, pango::FontDescription};
use libadwaita::{prelude::*, PreferencesWindow};

use crate::application::IvyApplication;

mod general;
mod keybindings;

pub const INITIAL_WIDTH: i32 = 802;
pub const INITIAL_HEIGHT: i32 = 648;
pub const APPLICATION_TITLE: &str = "ivyTerm";
pub const SPLIT_HANDLE_WIDTH: i32 = 10;
pub const SPLIT_VISUAL_WIDTH: i32 = 3;

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

impl GlobalConfig {
    pub fn get_terminal_config(&self) -> (FontDescription, [RGBA; 2], [RGBA; 16], u32) {
        let font_desc = self.font_desc.clone();
        let main_colors = self.main_colors.clone();
        let palette_colors = self.palette_colors.clone();
        let scrollback_lines = self.scrollback_lines.clone();

        (font_desc, main_colors, palette_colors, scrollback_lines)
    }
}

pub fn show_preferences_window(app: &IvyApplication) {
    // If a Settings window is already open, simply bring it to the front
    for window in app.windows() {
        if let Ok(window) = window.downcast::<PreferencesWindow>() {
            println!("Presenting an already open Settings window");
            window.present();
            return;
        }
    }

    let window = PreferencesWindow::builder().application(app).build();

    let general_page = create_general_page(app);
    window.add(&general_page);

    window.present();
}
