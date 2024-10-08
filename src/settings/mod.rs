use std::{fs, io::Write, path::PathBuf};

use default::{
    default_background, default_bright_colors, default_font, default_foreground,
    default_scrollback_lines, default_standard_colors,
};
use gtk4::{gdk::RGBA, pango::FontDescription};
use libadwaita::{prelude::*, PreferencesWindow};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use general::create_general_page;
use keybindings::create_keybinding_page;

use crate::{application::IvyApplication, keyboard::Keybindings};

mod default;
mod general;
mod keybindings;

pub const INITIAL_WIDTH: i32 = 802;
pub const INITIAL_HEIGHT: i32 = 648;
pub const APPLICATION_TITLE: &str = "ivyTerm";
pub const SPLIT_HANDLE_WIDTH: i32 = 10;
pub const SPLIT_VISUAL_WIDTH: i32 = 3;

#[derive(Deserialize, Serialize)]
pub struct GlobalConfig {
    #[serde(default, skip)]
    path: Option<PathBuf>,
    #[serde(default = "default_font")]
    pub font: IvyFont,
    #[serde(default = "default_scrollback_lines")]
    pub scrollback_lines: u32,
    #[serde(default = "default_foreground")]
    pub foreground: IvyColor,
    #[serde(default = "default_background")]
    pub background: IvyColor,
    #[serde(default = "default_standard_colors")]
    pub standard_colors: [IvyColor; 8],
    #[serde(default = "default_bright_colors")]
    pub bright_colors: [IvyColor; 8],
    #[serde(default)]
    pub keybindings: Keybindings,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        // Load user config
        if let Some(home_dir) = dirs::home_dir() {
            let parent_dir = home_dir.join(".config").join("ivyterm");
            let config_path = parent_dir.join("config.toml");
            let mut config: GlobalConfig = if config_path.exists() {
                // Config already exists, simply load it
                let config = fs::read_to_string(&config_path).unwrap();
                toml::from_str(&config).unwrap()
            } else {
                // We know we will be writing config back to file, ensure the parent directory exists
                fs::create_dir_all(parent_dir).unwrap();
                // Config doesn't yet exist, load default values
                toml::from_str("").unwrap()
            };
            // Store the path in config, so we don't have to determine it every time
            config.path = Some(config_path.clone());

            // Write parsed config back to the same path
            config.write_config_to_file();

            config
        } else {
            toml::from_str("").unwrap()
        }
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

    pub fn write_config_to_file(&self) {
        // Filesystem is always done async
        if let Some(path) = &self.path {
            let path = path.clone();
            let toml = toml::to_string(self).unwrap();

            glib::spawn_future_local(async move {
                let mut file = fs::File::create(path).expect("Unable to create config file");
                file.write_all(toml.as_bytes()).unwrap();
                file.flush().unwrap();
            });
        }
    }
}

#[derive(Clone)]
pub struct IvyColor(RGBA);

impl<'de> Deserialize<'de> for IvyColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex = String::deserialize(deserializer)?;
        match RGBA::parse(hex) {
            Ok(rgba) => Ok(IvyColor(rgba)),
            Err(err) => panic!("Error parsing hex: {}", err),
        }
    }
}

impl Serialize for IvyColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = self.to_hex();
        serializer.serialize_str(&string)
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

#[derive(Clone, Default)]
pub struct IvyFont(FontDescription);

impl IvyFont {
    pub fn new(font: &str) -> Self {
        let font = FontDescription::from_string(font);
        Self(font)
    }
}

impl<'de> Deserialize<'de> for IvyFont {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let font_description = String::deserialize(deserializer)?;
        let font_description = FontDescription::from_string(&font_description);
        Ok(IvyFont(font_description))
    }
}

impl Serialize for IvyFont {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string = self.0.to_str();
        serializer.serialize_str(&string)
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

    let keybinding_page = create_keybinding_page(app);
    window.add(&keybinding_page);

    window.present();
}
