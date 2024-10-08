use gtk4::gdk::RGBA;

use super::{IvyColor, IvyFont};

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
