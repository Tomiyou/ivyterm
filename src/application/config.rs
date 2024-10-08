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
        let app = self.clone();
        let window_box = Box::new(Orientation::Vertical, 0);

        // Window handle and buttons
        let header_bar = HeaderBar::new();

        // Font picker
        let font_dialog = FontDialog::new();
        let font_dialog_button = FontDialogButton::new(Some(font_dialog));
        font_dialog_button.connect_font_desc_notify(glib::clone!(
            #[weak]
            app,
            move |button| {
                let font_description = button.font_desc().unwrap();
                update_font(&app, font_description);
            }
        ));

        // Color pickers
        let foreground_color_dialog = ColorDialog::new();
        let foreground_color_button = ColorDialogButton::new(Some(foreground_color_dialog));
        foreground_color_button.connect_rgba_notify(glib::clone!(
            #[weak]
            app,
            move |button| {
                let rgba = button.rgba();
                update_foreground_color(&app, rgba)
            }
        ));

        let background_color_dialog = ColorDialog::new();
        let background_color_button = ColorDialogButton::new(Some(background_color_dialog));
        background_color_button.connect_rgba_notify(move |button| {
            let rgba = button.rgba();
            update_background_color(&app, rgba)
        });

        window_box.append(&header_bar);
        window_box.append(&font_dialog_button);
        window_box.append(&foreground_color_button);
        window_box.append(&background_color_button);

        // Create a new window
        let window = ApplicationWindow::builder()
            .application(self)
            .title(APPLICATION_TITLE)
            .content(&window_box)
            .build();

        window.present();
    }
}

fn update_foreground_color(app: &IvyApplication, rgba: RGBA) {
    let mut config = app.imp().config.borrow_mut();
    let [_foreground, background] = config.main_colors;
    config.main_colors = [rgba, background];
    drop(config);

    app.reload_css_colors();
}

fn update_background_color(app: &IvyApplication, rgba: RGBA) {
    let mut config = app.imp().config.borrow_mut();
    let [foreground, _background] = config.main_colors;
    config.main_colors = [foreground, rgba];
    drop(config);

    app.reload_css_colors();
}

fn update_font(app: &IvyApplication, font_desc: FontDescription) {
    let mut config = app.imp().config.borrow_mut();
    config.font_desc = font_desc;
    drop(config);

    app.refresh_terminals();
}
