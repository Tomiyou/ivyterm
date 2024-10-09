mod keybindings;

pub use keybindings::{check_keybinding_match, Keybinding, Keybindings};

#[derive(Clone, PartialEq, Debug)]
pub enum Direction {
    Left,
    Up,
    Right,
    Down,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeyboardAction {
    TabNew,
    TabClose,
    PaneSplit(bool),
    PaneClose,
    SelectPane(Direction),
    ToggleZoom,
    CopySelected,
}

pub fn keycode_to_arrow_key(keycode: u32) -> Option<Direction> {
    match keycode {
        111 => Some(Direction::Up),
        113 => Some(Direction::Left),
        114 => Some(Direction::Right),
        116 => Some(Direction::Down),
        _ => None,
    }
}
