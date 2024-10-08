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
}

pub fn handle_input(keycode: u32, state: ModifierType) -> Option<Keybinding> {
    if state.contains(ModifierType::CONTROL_MASK) && state.contains(ModifierType::SHIFT_MASK) {
        match keycode {
            28 => return Some(Keybinding::TabNew),
            25 => return Some(Keybinding::TabClose),
            32 => return Some(Keybinding::PaneSplit(true)),
            46 => return Some(Keybinding::PaneSplit(false)),
            26 => return Some(Keybinding::PaneClose),
            53 => return Some(Keybinding::ToggleZoom),
            _ => {}
        };
    }

    if state.contains(ModifierType::ALT_MASK) {
        match keycode {
            111 => return Some(Keybinding::SelectPane(Direction::Up)),
            113 => return Some(Keybinding::SelectPane(Direction::Left)),
            114 => return Some(Keybinding::SelectPane(Direction::Right)),
            116 => return Some(Keybinding::SelectPane(Direction::Down)),
            _ => {}
        }
    }

    None
}
