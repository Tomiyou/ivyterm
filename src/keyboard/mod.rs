mod keybindings;

pub use keybindings::{check_keybinding_match, Keybinding, Keybindings};

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum Direction {
    Left,
    Up,
    Right,
    Down,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum KeyboardAction {
    TabNew,
    TabClose,
    TabRename,
    PaneSplit(bool),
    PaneClose,
    // TODO: Correct naming
    MoveFocus(Direction),
    ToggleZoom,
    CopySelected,
    PasteClipboard,
    OpenEditorCwd,
    ClearScrollback,
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
