use gtk4::{
    gdk::{Event, KeyMatch, ModifierType},
    ShortcutTrigger,
};
use serde::{Deserialize, Serialize};
use vte4::ShortcutTriggerExt;

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

#[derive(Clone)]
pub struct Keybinding {
    pub trigger: Option<ShortcutTrigger>,
    pub action: KeyboardAction,
    pub description: &'static str,
}

impl Keybinding {
    fn new(trigger: &str, action: KeyboardAction, description: &'static str) -> Self {
        Self {
            trigger: ShortcutTrigger::parse_string(trigger),
            action: action,
            description: description,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Keybindings {
    new_tab: String,
    close_tab: String,
    split_horizontal: String,
    split_vertical: String,
    close_pane: String,
    toggle_zoom: String,
    copy_selection: String,
    move_right: String,
    move_left: String,
    move_up: String,
    move_down: String,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            new_tab: "<Ctrl><Shift>t".to_string(),
            close_tab: "<Ctrl><Shift>w".to_string(),
            split_horizontal: "<Ctrl><Shift>o".to_string(),
            split_vertical: "<Ctrl><Shift>l".to_string(),
            close_pane: "<Ctrl><Shift>d".to_string(),
            toggle_zoom: "<Ctrl><Shift>x".to_string(),
            copy_selection: "<Ctrl><Shift>c".to_string(),
            move_right: "<Alt>Right".to_string(),
            move_left: "<Alt>Left".to_string(),
            move_up: "<Alt>Up".to_string(),
            move_down: "<Alt>Down".to_string(),
        }
    }
}

impl Keybindings {
    pub fn init(&mut self) -> Vec<Keybinding> {
        let mut keybindings = Vec::new();

        // Put the most common keybindings first (for optimization)
        keybindings.push(Keybinding::new(
            &self.move_right,
            KeyboardAction::SelectPane(Direction::Right),
            "Move focus to the Pane on the right",
        ));
        keybindings.push(Keybinding::new(
            &self.move_left,
            KeyboardAction::SelectPane(Direction::Left),
            "Move focus to the Pane on the left",
        ));
        keybindings.push(Keybinding::new(
            &self.move_up,
            KeyboardAction::SelectPane(Direction::Up),
            "Move focus to the Pane on the up",
        ));
        keybindings.push(Keybinding::new(
            &self.move_down,
            KeyboardAction::SelectPane(Direction::Down),
            "Move focus to the Pane on the down",
        ));

        keybindings.push(Keybinding::new(
            &self.toggle_zoom,
            KeyboardAction::ToggleZoom,
            "Toggle zoom for the current Pane",
        ));
        keybindings.push(Keybinding::new(
            &self.copy_selection,
            KeyboardAction::CopySelected,
            "Copy selected text on the current Pane",
        ));

        keybindings.push(Keybinding::new(
            &self.new_tab,
            KeyboardAction::TabNew,
            "Open a new Tab",
        ));
        keybindings.push(Keybinding::new(
            &self.close_tab,
            KeyboardAction::TabClose,
            "Close the current Tab",
        ));
        keybindings.push(Keybinding::new(
            &self.split_horizontal,
            KeyboardAction::PaneSplit(true),
            "Split the current Tab horizontally",
        ));
        keybindings.push(Keybinding::new(
            &self.split_vertical,
            KeyboardAction::PaneSplit(false),
            "Split the current Tab vertically",
        ));
        keybindings.push(Keybinding::new(
            &self.close_pane,
            KeyboardAction::PaneClose,
            "Close the current Pane",
        ));

        keybindings
    }
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

#[inline]
pub fn check_keybinding_match(
    keybindings: &Vec<Keybinding>,
    event: Event,
) -> Option<KeyboardAction> {
    let state = event.modifier_state();
    if !state.contains(ModifierType::CONTROL_MASK)
        && !state.contains(ModifierType::SHIFT_MASK)
        && !state.contains(ModifierType::ALT_MASK)
    {
        return None;
    }

    for keybinding in keybindings {
        if let Some(trigger) = &keybinding.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(keybinding.action.clone());
            };
        }
    }

    None
}
