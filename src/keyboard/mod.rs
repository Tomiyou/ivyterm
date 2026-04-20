mod keybindings;

use gtk4::gdk::{Key, ModifierType};
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
    ToggleFullscreen,
    CopySelected,
    PasteClipboard,
    OpenEditorCwd,
    ClearScrollback,
}

pub fn gtk_key_to_tmux(cmd: &mut String, keycode: u32, keyval: Key, state: ModifierType) -> bool {
    use std::fmt::Write;

    #[inline]
    fn push_esc_sequence(cmd: &mut String, str: &str) {
        for c in str.chars() {
            write!(cmd, " {:#X}", c as u32).unwrap();
        }
    }

    if let Some(unicode) = keyval.to_unicode() {
        // Printable character
        let mut c = unicode as u32;

        if state.contains(ModifierType::ALT_MASK) {
            cmd.push_str(" 0x1B");
        }

        if state.contains(ModifierType::CONTROL_MASK) {
            c = c & 0x1F;
        }

        // cmd.push_str(string);
        write!(cmd, " {:#X}", c).unwrap();
    } else {
        // Write ESC[
        write!(cmd, " 0x1B 0x5B").unwrap();

        let mut modifier = 0;
        if state.contains(ModifierType::SHIFT_MASK) {
            modifier += 1;
        }
        if state.contains(ModifierType::ALT_MASK) {
            modifier += 2;
        }
        if state.contains(ModifierType::CONTROL_MASK) {
            modifier += 4;
        }
        if modifier > 0 {
            // Write 1; and modifier in ASCII (1 to 8)
            // modifier + 1 + 48 (where numbers start in ASCII)
            let modifier = modifier + 49;
            write!(cmd, " 0x31 0x3B {:#X}", modifier).unwrap();
        }

        // Non-printable character
        match keycode {
            // Up
            111 => push_esc_sequence(cmd, "A"),
            // Left
            113 => push_esc_sequence(cmd, "D"),
            // Right
            114 => push_esc_sequence(cmd, "C"),
            // Down
            116 => push_esc_sequence(cmd, "B"),
            // PageUp
            112 => push_esc_sequence(cmd, "5~"),
            // PageDown
            117 => push_esc_sequence(cmd, "6~"),
            // Home
            110 => push_esc_sequence(cmd, "H"),
            // End
            115 => push_esc_sequence(cmd, "F"),
            // Insert
            118 => push_esc_sequence(cmd, "2~"),
            // Delete
            119 => push_esc_sequence(cmd, "3~"),
            // F1
            67 => push_esc_sequence(cmd, ""),
            // F2
            68 => push_esc_sequence(cmd, ""),
            // F3
            69 => push_esc_sequence(cmd, ""),
            // F4
            70 => push_esc_sequence(cmd, ""),
            // F5
            71 => push_esc_sequence(cmd, "15~"),
            // F6
            72 => push_esc_sequence(cmd, "17~"),
            // F7
            73 => push_esc_sequence(cmd, "18~"),
            // F8
            74 => push_esc_sequence(cmd, "19~"),
            // F9
            75 => push_esc_sequence(cmd, "20~"),
            // F1
            76 => push_esc_sequence(cmd, "21~"),
            // F1
            95 => push_esc_sequence(cmd, "23~"),
            // F1
            96 => push_esc_sequence(cmd, "24~"),

            _ => return false,
        };
    }

    true
}
