mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Propagation, SpawnFlags};
use gtk4::{
    gdk::{ModifierType, RGBA},
    EventControllerKey, Orientation, ScrolledWindow,
};
use libadwaita::{glib, prelude::*};
use vte4::{PtyFlags, Terminal, TerminalExt, TerminalExtManual};

use crate::{
    global_state::GLOBAL_SETTINGS,
    keyboard::{handle_input, Keybinding},
    toplevel::TopLevel,
};

glib::wrapper! {
    pub struct IvyTerminal(ObjectSubclass<imp::IvyTerminalPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl IvyTerminal {
    pub fn new(top_level: &TopLevel) -> Self {
        let top_level = top_level.clone();

        // Get terminal font
        let (font_desc, [foreground, background], palette, scrollback_lines) = {
            let reader = GLOBAL_SETTINGS.read().unwrap();
            (
                reader.font_desc.clone(),
                reader.main_colors.clone(),
                reader.palette_colors.clone(),
                reader.scrollback_lines,
            )
        };

        let vte = Terminal::builder()
            .vexpand(true)
            .hexpand(true)
            .font_desc(&font_desc)
            .scrollback_lines(scrollback_lines)
            .build();

        // Add scrollbar
        let scrolled = ScrolledWindow::builder()
            .child(&vte)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Always)
            .build();

        // Create self
        let terminal: Self = Object::builder().build();
        terminal.set_child(Some(&scrolled));
        terminal.imp().set_terminal(&vte);

        // Add terminal to top level terminal list
        top_level.register_terminal(&terminal);

        // Close terminal + pane/tab when the child (shell) exits
        let _top_level = top_level.clone();
        let _terminal = terminal.clone();
        vte.connect_child_exited(move |_, _| {
            // Now close the pane/tab
            _top_level.close_pane(&_terminal);

            // Remove terminal from top level terminal list
            _top_level.unregister_terminal(&_terminal);
        });

        // Set terminal colors
        let palette: Vec<&RGBA> = palette.iter().map(|c| c).collect();
        vte.set_colors(Some(&foreground), Some(&background), &palette[..]);

        let eventctl = EventControllerKey::new();
        let _terminal = terminal.clone();
        eventctl.connect_key_pressed(move |_eventctl, _keyval, key, state| {
            handle_keyboard(key, state, &_terminal, &top_level)
        });
        vte.add_controller(eventctl);

        // Spawn terminal
        let pty_flags = PtyFlags::DEFAULT;
        let argv = ["/bin/bash"];
        let envv = [];
        let spawn_flags = SpawnFlags::DEFAULT;

        let _terminal = vte.clone();
        vte.spawn_async(
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
}

#[inline]
fn handle_keyboard(
    keycode: u32,
    state: ModifierType,
    terminal: &IvyTerminal,
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
