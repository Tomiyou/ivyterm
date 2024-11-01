use gtk4::gdk::RGBA;
use serde::{Deserialize, Serialize};

use super::IvyColor;

#[derive(Deserialize, Serialize, Clone)]
pub struct TmuxConfig {
    #[serde(default = "default_window_color")]
    pub window_color: IvyColor,
}

impl Default for TmuxConfig {
    fn default() -> Self {
        Self {
            window_color: default_window_color(),
        }
    }
}

pub fn default_window_color() -> IvyColor {
    let rgba = RGBA::parse("#420a42").unwrap();
    IvyColor(rgba)
}
