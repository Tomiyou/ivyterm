use gtk4::{
    gdk::{Event, KeyMatch, ModifierType},
    ShortcutTrigger,
};
use serde::{Deserialize, Deserializer};
use vte4::ShortcutTriggerExt;

#[derive(Clone, PartialEq)]
pub enum Direction {
    Left,
    Up,
    Right,
    Down,
}

#[derive(Clone)]
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
    pub text: String,
    pub trigger: Option<ShortcutTrigger>,
    pub action: KeyboardAction,
    pub description: &'static str,
}

impl<'de> serde::Deserialize<'de> for Keybinding {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Ok(Keybinding {
            text: text,
            trigger: None,
            action: KeyboardAction::TabClose,
            description: "",
        })
    }
}

impl Keybinding {
    fn parse(&mut self, action: KeyboardAction, description: &'static str) -> Self {
        self.trigger = ShortcutTrigger::parse_string(&self.text);
        self.action = action;
        self.description = description;
        self.clone()
    }
}

#[derive(Deserialize)]
pub struct Keybindings {
    new_tab: Keybinding,
    close_tab: Keybinding,
    split_horizontal: Keybinding,
    split_vertical: Keybinding,
    close_pane: Keybinding,
    toggle_zoom: Keybinding,
    copy_selection: Keybinding,
    move_right: Keybinding,
    move_left: Keybinding,
    move_up: Keybinding,
    move_down: Keybinding,
}

impl Keybindings {
    pub fn init(&mut self) {
        self.new_tab.parse(KeyboardAction::TabNew, "Open a new Tab");
        self.close_tab
            .parse(KeyboardAction::TabClose, "Close the current Tab");
        self.split_horizontal.parse(
            KeyboardAction::PaneSplit(true),
            "Split the current Tab horizontally",
        );
        self.split_vertical.parse(
            KeyboardAction::PaneSplit(false),
            "Split the current Tab vertically",
        );
        self.close_pane
            .parse(KeyboardAction::PaneClose, "Close the current Pane");
        self.toggle_zoom.parse(
            KeyboardAction::ToggleZoom,
            "Toggle zoom for the current Pane",
        );
        self.copy_selection.parse(
            KeyboardAction::CopySelected,
            "Copy selected text on the current Pane",
        );
        self.move_right.parse(
            KeyboardAction::SelectPane(Direction::Right),
            "Move focus to the Pane on the right",
        );
        self.move_left.parse(
            KeyboardAction::SelectPane(Direction::Left),
            "Move focus to the Pane on the left",
        );
        self.move_up.parse(
            KeyboardAction::SelectPane(Direction::Up),
            "Move focus to the Pane on the up",
        );
        self.move_down.parse(
            KeyboardAction::SelectPane(Direction::Down),
            "Move focus to the Pane on the down",
        );
    }

    pub fn handle_event(&self, event: Event) -> Option<KeyboardAction> {
        let state = event.modifier_state();
        if !state.contains(ModifierType::CONTROL_MASK)
            && !state.contains(ModifierType::SHIFT_MASK)
            && !state.contains(ModifierType::ALT_MASK)
        {
            return None;
        }

        // Movement shortcuts
        if let Some(trigger) = &self.move_right.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.move_right.action.clone());
            };
        }
        if let Some(trigger) = &self.move_left.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.move_left.action.clone());
            };
        }
        if let Some(trigger) = &self.move_up.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.move_up.action.clone());
            };
        }
        if let Some(trigger) = &self.move_down.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.move_down.action.clone());
            };
        }

        // Copy and zoom
        if let Some(trigger) = &self.toggle_zoom.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.toggle_zoom.action.clone());
            };
        }
        if let Some(trigger) = &self.copy_selection.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.copy_selection.action.clone());
            };
        }

        // Pane manipulation
        if let Some(trigger) = &self.new_tab.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.new_tab.action.clone());
            };
        }
        if let Some(trigger) = &self.close_tab.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.close_tab.action.clone());
            };
        }
        if let Some(trigger) = &self.split_horizontal.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.split_horizontal.action.clone());
            };
        }
        if let Some(trigger) = &self.split_vertical.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.split_vertical.action.clone());
            };
        }
        if let Some(trigger) = &self.close_pane.trigger {
            if trigger.trigger(&event, true) == KeyMatch::Exact {
                return Some(self.close_pane.action.clone());
            };
        }

        None
    }

    pub fn to_vec(&self) -> Vec<Keybinding> {
        let mut ret = Vec::new();

        ret.push(self.new_tab.clone());
        ret.push(self.close_tab.clone());
        ret.push(self.split_horizontal.clone());
        ret.push(self.split_vertical.clone());
        ret.push(self.close_pane.clone());
        ret.push(self.toggle_zoom.clone());
        ret.push(self.copy_selection.clone());
        ret.push(self.move_right.clone());
        ret.push(self.move_left.clone());
        ret.push(self.move_up.clone());
        ret.push(self.move_down.clone());
        ret
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
