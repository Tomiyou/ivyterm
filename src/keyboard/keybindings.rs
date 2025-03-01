use gtk4::{
    gdk::{Event, KeyMatch, ModifierType},
    ShortcutTrigger,
};
use libadwaita::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Direction, KeyboardAction};

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

#[derive(Deserialize, Serialize, Clone)]
pub struct Keybindings {
    #[serde(default = "default_new_tab")]
    new_tab: String,
    #[serde(default = "default_close_tab")]
    close_tab: String,
    #[serde(default = "default_split_horizontal")]
    split_horizontal: String,
    #[serde(default = "default_split_vertical")]
    split_vertical: String,
    #[serde(default = "default_close_pane")]
    close_pane: String,
    #[serde(default = "default_toggle_zoom")]
    toggle_zoom: String,
    #[serde(default = "default_copy_selection")]
    copy_selection: String,
    #[serde(default = "default_move_right")]
    move_right: String,
    #[serde(default = "default_move_left")]
    move_left: String,
    #[serde(default = "default_move_up")]
    move_up: String,
    #[serde(default = "default_move_down")]
    move_down: String,
    #[serde(default = "default_rename_tab")]
    rename_tab: String,
    #[serde(default = "default_paste_clipboard")]
    paste_clipboard: String,
    #[serde(default = "default_open_editor_cwd")]
    open_editor_cwd: String,
    #[serde(default = "default_clear_scrollback")]
    clear_scrollback: String,
}

impl Keybindings {
    pub fn init(&mut self) -> Vec<Keybinding> {
        let mut keybindings = Vec::new();

        // Put the most common keybindings first (for optimization)
        keybindings.push(Keybinding::new(
            &self.move_right,
            KeyboardAction::MoveFocus(Direction::Right),
            "Move focus to the Terminal on the right",
        ));
        keybindings.push(Keybinding::new(
            &self.move_left,
            KeyboardAction::MoveFocus(Direction::Left),
            "Move focus to the Terminal on the left",
        ));
        keybindings.push(Keybinding::new(
            &self.move_up,
            KeyboardAction::MoveFocus(Direction::Up),
            "Move focus to the Terminal on the up",
        ));
        keybindings.push(Keybinding::new(
            &self.move_down,
            KeyboardAction::MoveFocus(Direction::Down),
            "Move focus to the Terminal on the down",
        ));

        keybindings.push(Keybinding::new(
            &self.toggle_zoom,
            KeyboardAction::ToggleZoom,
            "Toggle zoom for the current Terminal",
        ));
        keybindings.push(Keybinding::new(
            &self.copy_selection,
            KeyboardAction::CopySelected,
            "Copy selected text on the current Terminal",
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
            "Close the current Terminal",
        ));
        keybindings.push(Keybinding::new(
            &self.rename_tab,
            KeyboardAction::TabRename,
            "Rename the current Tab",
        ));
        keybindings.push(Keybinding::new(
            &self.paste_clipboard,
            KeyboardAction::PasteClipboard,
            "Paste to Terminal",
        ));
        keybindings.push(Keybinding::new(
            &self.open_editor_cwd,
            KeyboardAction::OpenEditorCwd,
            "Open current working directory in VS Code",
        ));
        keybindings.push(Keybinding::new(
            &self.clear_scrollback,
            KeyboardAction::ClearScrollback,
            "Clear Tmux scrollback",
        ));

        keybindings
    }

    pub fn update(&mut self, keybindings: &Vec<Keybinding>) {
        for keybinding in keybindings {
            let trigger = if let Some(trigger) = &keybinding.trigger {
                let trigger = trigger.to_str();
                trigger.to_string()
            } else {
                "".to_string()
            };

            match keybinding.action {
                KeyboardAction::MoveFocus(Direction::Right) => self.move_right = trigger,
                KeyboardAction::MoveFocus(Direction::Left) => self.move_left = trigger,
                KeyboardAction::MoveFocus(Direction::Up) => self.move_up = trigger,
                KeyboardAction::MoveFocus(Direction::Down) => self.move_down = trigger,
                KeyboardAction::ToggleZoom => self.toggle_zoom = trigger,
                KeyboardAction::CopySelected => self.copy_selection = trigger,
                KeyboardAction::TabNew => self.new_tab = trigger,
                KeyboardAction::TabClose => self.close_tab = trigger,
                KeyboardAction::TabRename => self.rename_tab = trigger,
                KeyboardAction::PaneSplit(true) => self.split_horizontal = trigger,
                KeyboardAction::PaneSplit(false) => self.split_vertical = trigger,
                KeyboardAction::PaneClose => self.close_pane = trigger,
                KeyboardAction::PasteClipboard => self.paste_clipboard = trigger,
                KeyboardAction::OpenEditorCwd => self.open_editor_cwd = trigger,
                KeyboardAction::ClearScrollback => self.clear_scrollback = trigger,
            }
        }
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

// Default Keybindings
impl Default for Keybindings {
    fn default() -> Self {
        Self {
            new_tab: default_new_tab(),
            close_tab: default_close_tab(),
            split_horizontal: default_split_horizontal(),
            split_vertical: default_split_vertical(),
            close_pane: default_close_pane(),
            toggle_zoom: default_toggle_zoom(),
            copy_selection: default_copy_selection(),
            move_right: default_move_right(),
            move_left: default_move_left(),
            move_up: default_move_up(),
            move_down: default_move_down(),
            rename_tab: default_rename_tab(),
            paste_clipboard: default_paste_clipboard(),
            open_editor_cwd: default_open_editor_cwd(),
            clear_scrollback: default_clear_scrollback(),
        }
    }
}

fn default_new_tab() -> String {
    "<Ctrl><Shift>t".to_string()
}
fn default_close_tab() -> String {
    "<Ctrl><Shift>w".to_string()
}
fn default_split_horizontal() -> String {
    "<Ctrl><Shift>o".to_string()
}
fn default_split_vertical() -> String {
    "<Ctrl><Shift>l".to_string()
}
fn default_close_pane() -> String {
    "<Ctrl><Shift>d".to_string()
}
fn default_toggle_zoom() -> String {
    "<Ctrl><Shift>x".to_string()
}
fn default_copy_selection() -> String {
    "<Ctrl><Shift>c".to_string()
}
fn default_paste_clipboard() -> String {
    "<Ctrl><Shift>v".to_string()
}
fn default_move_right() -> String {
    "<Alt>Right".to_string()
}
fn default_move_left() -> String {
    "<Alt>Left".to_string()
}
fn default_move_up() -> String {
    "<Alt>Up".to_string()
}
fn default_move_down() -> String {
    "<Alt>Down".to_string()
}
fn default_rename_tab() -> String {
    "<Ctrl><Alt>A".to_string()
}
fn default_open_editor_cwd() -> String {
    "<Ctrl><Shift>m".to_string()
}
fn default_clear_scrollback() -> String {
    "<Ctrl><Shift>h".to_string()
}
