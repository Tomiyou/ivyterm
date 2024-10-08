use gtk4::gdk::ModifierType;

#[derive(PartialEq)]
pub enum Direction {
    Left,
    Up,
    Right,
    Down,
}

pub enum Keybinding {
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
        _ => None
    }
}

pub fn handle_input(keycode: u32, state: ModifierType) -> Option<Keybinding> {
    if state.contains(ModifierType::CONTROL_MASK) && state.contains(ModifierType::SHIFT_MASK) {
        // println!("Keycode is {}", keycode);
        match keycode {
            28 => return Some(Keybinding::TabNew),
            25 => return Some(Keybinding::TabClose),
            32 => return Some(Keybinding::PaneSplit(true)),
            46 => return Some(Keybinding::PaneSplit(false)),
            26 => return Some(Keybinding::PaneClose),
            53 => return Some(Keybinding::ToggleZoom),
            54 => return Some(Keybinding::CopySelected),
            _ => {}
        };
    }

    if state.contains(ModifierType::ALT_MASK) {
        if let Some(direction) = keycode_to_arrow_key(keycode) {
            return Some(Keybinding::SelectPane(direction))
        }
    }

    None
}
