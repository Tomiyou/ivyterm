use glib::{Propagation, SpawnFlags};
use gtk4::{gdk::{ModifierType, RGBA}, EventControllerKey, Orientation};
use libadwaita::prelude::*;
use vte4::{PtyFlags, Terminal, TerminalExt, TerminalExtManual};

use crate::{
    global_state::GLOBAL_SETTINGS,
    keyboard::{handle_input, Keybinding},
    mux::toplevel::TopLevel,
};

pub fn create_terminal(top_level: &TopLevel) -> Terminal {
    let top_level = top_level.clone();

    // Get terminal font
    let (font_desc, [foreground, background], palette) = {
        let reader = GLOBAL_SETTINGS.read().unwrap();
        (reader.font_desc.clone(), reader.main_colors.clone(), reader.palette_colors.clone())
    };

    let terminal = Terminal::builder()
        .vexpand(true)
        .hexpand(true)
        .font_desc(&font_desc)
        .build();

    // Add terminal to top level terminal list
    top_level.register_terminal(&terminal);

    // Close terminal + pane/tab when the child (shell) exits
    let _top_level = top_level.clone();
    terminal.connect_child_exited(move |terminal, _exit_code| {
        // Remove terminal from top level terminal list
        _top_level.unregister_terminal(terminal);

        // Now close the pane/tab
        _top_level.close_pane(terminal);
    });

    // Set terminal colors
    let palette: Vec<&RGBA> = palette.iter().map(|c| c).collect();
    terminal.set_colors(Some(&foreground), Some(&background), &palette[..]);

    let eventctl = EventControllerKey::new();
    let _terminal = terminal.clone();
    eventctl.connect_key_pressed(move |_eventctl, _keyval, key, state| {
        handle_keyboard(key, state, &_terminal, &top_level)
    });
    terminal.add_controller(eventctl);

    // Spawn terminal
    let pty_flags = PtyFlags::DEFAULT;
    let argv = ["/bin/bash"];
    let envv = [];
    let spawn_flags = SpawnFlags::DEFAULT;

    let _terminal = terminal.clone();
    terminal.spawn_async(
        pty_flags,
        None,
        &argv,
        &envv,
        spawn_flags,
        || {},
        -1,
        gtk4::gio::Cancellable::NONE,
        move |_result| {
            _terminal.grab_focus();
        },
    );

    terminal
}

#[inline]
fn handle_keyboard(
    keycode: u32,
    state: ModifierType,
    terminal: &Terminal,
    top_level: &TopLevel,
) -> Propagation {
    // Handle terminal splits
    if !state.contains(ModifierType::CONTROL_MASK)
        && !state.contains(ModifierType::SHIFT_MASK)
        && !state.contains(ModifierType::ALT_MASK)
    {
        return Propagation::Proceed;
    }

    let keyboard_action = handle_input(keycode, state);
    match keyboard_action {
        Some(Keybinding::PaneSplit(vertical)) => {
            let orientation = if vertical {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            };

            top_level.split_pane(terminal, orientation);
        }
        Some(Keybinding::PaneClose) => {
            top_level.close_pane(terminal);
        }
        Some(Keybinding::TabNew) => {
            top_level.create_tab();
        }
        Some(Keybinding::TabClose) => {
            top_level.close_tab();
        }
        Some(Keybinding::SelectPane(direction)) => {
            top_level.unzoom();
            if let Some(new_focus) = top_level.find_neighbor(terminal, direction) {
                new_focus.grab_focus();
            }
        }
        Some(Keybinding::ToggleZoom) => {
            top_level.toggle_zoom(terminal);
        }
        None => return Propagation::Proceed,
    };

    Propagation::Stop
}
