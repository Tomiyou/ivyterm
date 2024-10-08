use glib::{Propagation, SpawnFlags};
use gtk4::{
    gdk::{Key, ModifierType, RGBA},
    EventControllerKey, Orientation, Paned, Widget,
};
use libadwaita::prelude::*;
use vte4::{PtyFlags, Terminal, TerminalExt, TerminalExtManual};

use crate::{
    global_state::GLOBAL_SETTINGS,
    keyboard::{matches_keybinding, Keybinding},
    mux::{pane::close_pane, toplevel::TopLevel},
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
    terminal.connect_child_exited(|terminal, _exit_code| {
        println!("Exited!");
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
    eventctl.connect_key_pressed(move |eventctl, keyval, key, state| {
        handle_keyboard(eventctl, keyval, key, state, &top_level)
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
    eventctl: &EventControllerKey,
    keyval: Key,
    keycode: u32,
    state: ModifierType,
    top_level: &TopLevel,
) -> Propagation {
    // Handle terminal splits
    if !state.contains(ModifierType::CONTROL_MASK) || !state.contains(ModifierType::SHIFT_MASK) {
        return Propagation::Proceed;
    }

    println!("Terminal has keycode {}", keycode);

    // Split vertical
    if matches_keybinding(keyval, keycode, state, Keybinding::PaneSplit(true)) {
        match cast_parent(eventctl.widget().parent().unwrap()) {
            ParentType::ParentTopLevel(top_level) => top_level.split(Orientation::Vertical),
            ParentType::ParentPaned(paned) => split_pane(paned, Orientation::Vertical, top_level),
        }
        return Propagation::Stop;
    }

    // Split horizontal
    if matches_keybinding(keyval, keycode, state, Keybinding::PaneSplit(false)) {
        match cast_parent(eventctl.widget().parent().unwrap()) {
            ParentType::ParentTopLevel(top_level) => top_level.split(Orientation::Horizontal),
            ParentType::ParentPaned(paned) => split_pane(paned, Orientation::Horizontal, top_level),
        }
        return Propagation::Stop;
    }

    // Close pane
    if matches_keybinding(keyval, keycode, state, Keybinding::PaneClose) {
        match cast_parent(eventctl.widget().parent().unwrap()) {
            ParentType::ParentTopLevel(top_level) => top_level.close_tab(),
            ParentType::ParentPaned(paned) => close_pane(paned),
        }
        return Propagation::Stop;
    }

    // Create tab
    if matches_keybinding(keyval, keycode, state, Keybinding::TabNew) {
        top_level.create_tab();
        return Propagation::Stop;
    }

    // Close tab
    if matches_keybinding(keyval, keycode, state, Keybinding::TabClose) {
        top_level.close_tab();
        return Propagation::Stop;
    }

    Propagation::Proceed
}
