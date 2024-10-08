mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Propagation, SpawnFlags};
use gtk4::{
    gdk::{ModifierType, RGBA},
    EventControllerKey, Orientation, ScrolledWindow,
};
use libadwaita::{glib, prelude::*};
use vte4::{PtyFlags, Terminal as Vte, TerminalExt, TerminalExtManual};

use crate::{
    global_state::GLOBAL_SETTINGS,
    keyboard::{handle_input, Keybinding},
    next_unique_pane_id,
    toplevel::TopLevel,
    window::IvyWindow,
};

glib::wrapper! {
    pub struct Terminal(ObjectSubclass<imp::TerminalPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Terminal {
    pub fn new(top_level: &TopLevel, window: &IvyWindow, pane_id: Option<u32>) -> Self {
        let pane_id = match pane_id {
            Some(pane_id) => pane_id,
            None => next_unique_pane_id(),
        };

        let is_tmux = window.is_tmux();
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

        let vte = Vte::builder()
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
        terminal.imp().init_values(pane_id, &vte, window);

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
        let _window = window.clone();
        let _vte = vte.clone();
        eventctl.connect_key_pressed(move |_eventctl, keyval, key, state| {
            if is_tmux {
                _window.tmux_keypress(pane_id, key, keyval, state);
                Propagation::Proceed
            } else {
                handle_keyboard(key, state, &_terminal, &top_level, &_vte)
            }
        });
        vte.add_controller(eventctl);

        // Spawn terminal
        let pty_flags = PtyFlags::DEFAULT;
        let argv = ["/bin/bash"];
        let envv = [];
        let spawn_flags = SpawnFlags::DEFAULT;

        if !is_tmux {
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
        }

        terminal
    }

    pub fn pane_id(&self) -> u32 {
        self.imp().id.get()
    }

    pub fn feed_output(&self, output: Vec<u8>) {
        let binding = self.imp().vte.borrow();
        let vte = binding.as_ref().unwrap();
        vte.feed(&output);
    }

    pub fn scroll_view(&self, empty_lines: usize) {
        let mut output = Vec::with_capacity(empty_lines + 16);
        // Scroll down 'empty_lines' lines
        for _ in 0..empty_lines {
            output.push(b'\n');
        }
        // Scroll back up '# = empty_lines' lines using ESC[#A
        output.push(b'\x1b');
        output.push(b'[');
        for d in empty_lines.to_string().as_bytes() {
            output.push(*d);
        }
        output.push(b'A');

        self.feed_output(output);
    }

    pub fn get_rows_cols_for_size(&self, width: i32, height: i32) -> (i32, i32) {
        let binding = self.imp().vte.borrow();
        let vte = binding.as_ref().unwrap();

        let char_width = vte.char_width();
        let char_height = vte.char_height();

        let cols = (width as i64) / char_width;
        let rows = (height as i64) / char_height;
        (cols as i32, rows as i32)
    }
}

#[inline]
fn handle_keyboard(
    keycode: u32,
    state: ModifierType,
    terminal: &Terminal,
    top_level: &TopLevel,
    vte: &Vte,
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
            top_level.create_tab(None);
        }
        Some(Keybinding::TabClose) => {
            top_level.close_tab();
        }
        Some(Keybinding::SelectPane(direction)) => {
            let previous_size = top_level.unzoom();
            if let Some(new_focus) = top_level.find_neighbor(terminal, direction, previous_size) {
                new_focus.grab_focus();
            }
        }
        Some(Keybinding::ToggleZoom) => {
            top_level.toggle_zoom(terminal);
        }
        Some(Keybinding::CopySelected) => {
            vte.emit_copy_clipboard();
        }
        None => return Propagation::Proceed,
    };

    Propagation::Stop
}
