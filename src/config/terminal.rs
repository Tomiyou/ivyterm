use gtk4::gdk::RGBA;
use serde::{Deserialize, Serialize};

use super::{IvyColor, IvyFont};

#[derive(Deserialize, Serialize, Clone)]
pub struct TerminalConfig {
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
    #[serde(default = "default_split_handle_color")]
    pub split_handle_color: IvyColor,
    #[serde(default)]
    pub terminal_bell: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            font: default_font(),
            scrollback_lines: default_scrollback_lines(),
            foreground: default_foreground(),
            background: default_background(),
            standard_colors: default_standard_colors(),
            bright_colors: default_bright_colors(),
            split_handle_color: default_split_handle_color(),
            terminal_bell: false,
        }
    }
}

pub struct ColorScheme {
    colors: [RGBA; 16],
}

impl ColorScheme {
    pub fn new(config: &TerminalConfig) -> Self {
        let mut color_scheme: Vec<RGBA> = Vec::with_capacity(16);
        for color in &config.standard_colors {
            color_scheme.push(color.clone().into());
        }
        for color in &config.bright_colors {
            color_scheme.push(color.clone().into());
        }

        Self {
            colors: color_scheme.try_into().unwrap(),
        }
    }

    pub fn get(&self) -> Vec<&RGBA> {
        self.colors.iter().map(|c| c).collect()
    }
}

pub fn default_font() -> IvyFont {
    IvyFont::new("Monospace 12")
}

pub fn default_scrollback_lines() -> u32 {
    500
}

pub fn default_foreground() -> IvyColor {
    let rgba = RGBA::parse("#ffffff").unwrap();
    IvyColor(rgba)
}

pub fn default_background() -> IvyColor {
    let rgba = RGBA::parse("#000000").unwrap();
    IvyColor(rgba)
}

pub fn default_split_handle_color() -> IvyColor {
    let rgba = RGBA::parse("#ffffff").unwrap();
    IvyColor(rgba)
}

pub fn default_standard_colors() -> [IvyColor; 8] {
    [
        IvyColor(RGBA::parse("#2e3436").unwrap()),
        IvyColor(RGBA::parse("#cc0000").unwrap()),
        IvyColor(RGBA::parse("#4e9a06").unwrap()),
        IvyColor(RGBA::parse("#c4a000").unwrap()),
        IvyColor(RGBA::parse("#3465a4").unwrap()),
        IvyColor(RGBA::parse("#75507b").unwrap()),
        IvyColor(RGBA::parse("#06989a").unwrap()),
        IvyColor(RGBA::parse("#d3d7cf").unwrap()),
    ]
}

pub fn default_bright_colors() -> [IvyColor; 8] {
    [
        IvyColor(RGBA::parse("#555753").unwrap()),
        IvyColor(RGBA::parse("#ef2929").unwrap()),
        IvyColor(RGBA::parse("#8ae234").unwrap()),
        IvyColor(RGBA::parse("#fce94f").unwrap()),
        IvyColor(RGBA::parse("#729fcf").unwrap()),
        IvyColor(RGBA::parse("#ad7fa8").unwrap()),
        IvyColor(RGBA::parse("#34e2e2").unwrap()),
        IvyColor(RGBA::parse("#eeeeec").unwrap()),
    ]
}
