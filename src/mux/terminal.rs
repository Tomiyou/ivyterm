use std::sync::atomic::Ordering;

use glib::{Propagation, SpawnFlags};
use gtk4::{
    gdk::{ModifierType, RGBA},
    EventControllerKey, Orientation, Paned, Widget,
};
use libadwaita::prelude::*;
use vte4::{PtyFlags, Terminal, TerminalExt, TerminalExtManual};

use crate::{
    global_state::GLOBAL_SETTINGS,
    keyboard::{handle_input, Direction, Keybinding},
    mux::{pane::close_pane, toplevel::TopLevel},
    GLOBAL_TERMINAL_ID,
};

use super::pane::split_pane;

fn default_colors() -> (RGBA, RGBA) {
    let foreground = RGBA::new(1.0, 1.0, 1.0, 1.0);
    let background = RGBA::new(0.0, 0.0, 0.0, 1.0);

    (foreground, background)
}

enum ParentType {
    ParentPaned(Paned),
    ParentTopLevel(TopLevel),
}

fn cast_parent(parent: Widget) -> ParentType {
    if let Ok(paned) = parent.clone().downcast::<Paned>() {
        return ParentType::ParentPaned(paned);
    } else if let Ok(top_level) = parent.downcast::<TopLevel>() {
        return ParentType::ParentTopLevel(top_level);
    }

    panic!("Parent is neither Bin nor Paned")
}

pub fn create_terminal(top_level: &TopLevel) -> Terminal {
    let terminal_id = GLOBAL_TERMINAL_ID.fetch_add(1, Ordering::Relaxed);

    // Get terminal font
    let font_desc = {
        let reader = GLOBAL_SETTINGS.read().unwrap();
        reader.font_desc.clone()
    };

    let terminal = Terminal::builder()
        .vexpand(true)
        .hexpand(true)
        .font_desc(&font_desc)
        .build();

    // Close terminal + pane/tab when the child (shell) exits
    terminal.connect_child_exited(move |terminal, _exit_code| {
        println!("Terminal {} exited!", terminal_id);
        terminal.unrealize();

        let parent = terminal.parent().unwrap();
        match cast_parent(parent) {
            ParentType::ParentTopLevel(top_level) => top_level.close_tab(),
            ParentType::ParentPaned(paned) => close_pane(paned),
        }
    });

    // Set terminal colors
    let (foreground, background) = default_colors();
    terminal.set_colors(Some(&foreground), Some(&background), &[]);

    let eventctl = EventControllerKey::new();
    let top_level = top_level.clone();
    eventctl.connect_key_pressed(move |eventctl, _keyval, key, state| {
        handle_keyboard(eventctl, key, state, &top_level)
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
            println!("Terminal {} spawned, grabbing focus", terminal_id);
            _terminal.grab_focus();
        },
    );

    terminal
}

#[inline]
fn handle_keyboard(
    eventctl: &EventControllerKey,
    keycode: u32,
    state: ModifierType,
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
            match cast_parent(eventctl.widget().parent().unwrap()) {
                ParentType::ParentTopLevel(top_level) => top_level.split(orientation),
                ParentType::ParentPaned(paned) => split_pane(paned, orientation, top_level),
            }
        }
        Some(Keybinding::PaneClose) => match cast_parent(eventctl.widget().parent().unwrap()) {
            ParentType::ParentTopLevel(top_level) => top_level.close_tab(),
            ParentType::ParentPaned(paned) => close_pane(paned),
        },
        Some(Keybinding::TabNew) => {
            top_level.create_tab();
        }
        Some(Keybinding::TabClose) => {
            top_level.close_tab();
        }
        Some(Keybinding::SelectPane(direction)) => {
            match cast_parent(eventctl.widget().parent().unwrap()) {
                ParentType::ParentPaned(paned) => move_pane_focus(paned, direction),
                ParentType::ParentTopLevel(_) => {},
            };
        }
        None => return Propagation::Proceed,
    };

    Propagation::Stop
}

fn move_pane_focus(parent: Paned, direction: Direction) {
    match (parent.orientation(), direction) {
        (Orientation::Horizontal, Direction::Left) => if parent.last_child().unwrap().has_focus() {
            parent.emit_cycle_child_focus(true);
            return;
        },
        (Orientation::Horizontal, Direction::Right) => if parent.start_child().unwrap().has_focus() {
            parent.emit_cycle_child_focus(false);
            return;
        },
        (Orientation::Vertical, Direction::Up) => if parent.last_child().unwrap().has_focus() {
            parent.emit_cycle_child_focus(true);
            return;
        },
        (Orientation::Vertical, Direction::Down) => if parent.start_child().unwrap().has_focus() {
            parent.emit_cycle_child_focus(false);
            return;
        },
        _ => {},
    };

}
