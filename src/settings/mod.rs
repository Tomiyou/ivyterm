use gtk4::{gdk::RGBA, pango::FontDescription};
use libadwaita::{prelude::*, PreferencesWindow};
use serde::{Deserialize, Deserializer};

use general::create_general_page;
// use keybindings::create_keybinding_page;

use crate::{application::IvyApplication, keyboard::Keybindings};

mod general;
mod keybindings;

pub const INITIAL_WIDTH: i32 = 802;
pub const INITIAL_HEIGHT: i32 = 648;
pub const APPLICATION_TITLE: &str = "ivyTerm";
pub const SPLIT_HANDLE_WIDTH: i32 = 10;
pub const SPLIT_VISUAL_WIDTH: i32 = 3;

#[derive(Deserialize)]
pub struct GlobalConfig {
    pub font: IvyFont,
    pub scrollback_lines: u32,
    pub foreground: IvyColor,
    pub background: IvyColor,
    pub standard_colors: [IvyColor; 8],
    pub bright_colors: [IvyColor; 8],
    pub keybindings: Keybindings,
}

#[derive(Clone)]
pub struct IvyColor(RGBA);
impl<'de> serde::Deserialize<'de> for IvyColor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(d)?;
        match RGBA::parse(hex) {
            Ok(rgba) => Ok(IvyColor(rgba)),
            Err(err) => panic!("Error parsing hex: {}", err),
        }
    }
}

impl IvyColor {
    pub fn to_hex(&self) -> String {
        let rgba = self.0;
        let red = (rgba.red() * 255.).round() as i32;
        let green = (rgba.green() * 255.).round() as i32;
        let blue = (rgba.blue() * 255.).round() as i32;
        format!("#{:02X}{:02X}{:02X}", red, green, blue)
    }
}

impl From<RGBA> for IvyColor {
    fn from(value: RGBA) -> Self {
        Self(value)
    }
}

impl Into<RGBA> for IvyColor {
    fn into(self) -> RGBA {
        self.0
    }
}

#[derive(Clone)]
pub struct IvyFont(FontDescription);
impl<'de> serde::Deserialize<'de> for IvyFont {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let font_description = String::deserialize(d)?;
        let font_description = FontDescription::from_string(&font_description);
        Ok(IvyFont(font_description))
    }
}

impl From<FontDescription> for IvyFont {
    fn from(value: FontDescription) -> Self {
        Self(value)
    }
}

impl Into<FontDescription> for IvyFont {
    fn into(self) -> FontDescription {
        self.0
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let default_config = include_str!("../default.toml");
        let config: GlobalConfig = toml::from_str(&default_config).unwrap();
        config
    }
}

impl GlobalConfig {
    pub fn get_terminal_config(&self) -> (FontDescription, [RGBA; 2], [RGBA; 16], u32) {
        let font = self.font.clone().into();
        let scrollback_lines = self.scrollback_lines.clone();
        let main_colors = [
            self.foreground.clone().into(),
            self.background.clone().into(),
        ];
        let standard_colors: [RGBA; 8] = self.standard_colors.clone().map(|c| c.into());
        let bright_colors: [RGBA; 8] = self.bright_colors.clone().map(|c| c.into());

        let palette_colors = [standard_colors, bright_colors].concat();
        let palette_colors: [RGBA; 16] = palette_colors.try_into().unwrap();

        (font, main_colors, palette_colors, scrollback_lines)
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

    // let keybinding_page = create_keybinding_page(app);
    // window.add(&keybinding_page);

    window.present();
}
