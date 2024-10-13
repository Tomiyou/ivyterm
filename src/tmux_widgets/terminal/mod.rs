mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Propagation};
use gtk4::{gdk::RGBA, pango::FontDescription, EventControllerKey, ScrolledWindow};
use libadwaita::{glib, prelude::*};
use vte4::{Terminal as Vte, TerminalExt, TerminalExtManual};

use crate::{application::IvyApplication, keyboard::KeyboardAction};

use super::{toplevel::TmuxTopLevel, IvyTmuxWindow};

glib::wrapper! {
    pub struct TmuxTerminal(ObjectSubclass<imp::TerminalPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl TmuxTerminal {
    pub fn new(top_level: &TmuxTopLevel, window: &IvyTmuxWindow, pane_id: u32) -> Self {
        let window = window.clone();

        let app = window.application().unwrap();
        let app: IvyApplication = app.downcast().unwrap();

        // Get terminal font
        let (font_desc, [foreground, background], palette, scrollback_lines) =
            app.get_terminal_config();

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
        terminal.imp().init_values(pane_id, &vte, &window);

        if window.initial_output_finished() {
            terminal.imp().set_synced();
        }

        // Add terminal to top level terminal list
        top_level.register_terminal(&terminal);

        // Set terminal colors
        let palette: Vec<&RGBA> = palette.iter().map(|c| c).collect();
        vte.set_colors(Some(&foreground), Some(&background), &palette[..]);

        vte.connect_has_focus_notify(glib::clone!(
            #[strong]
            top_level,
            move |vte| {
                if vte.has_focus() {
                    // Notify TopLevel that the focused terminal changed
                    top_level.terminal_focus_changed(pane_id);
                }
            }
        ));

        let eventctl = EventControllerKey::new();
        eventctl.connect_key_pressed(glib::clone!(
            #[weak]
            vte,
            #[upgrade_or]
            Propagation::Proceed,
            move |eventctl, keyval, key, state| {
                if let Some(event) = eventctl.current_event() {
                    // Check if pressed keys match a keybinding
                    if let Some(action) = app.handle_keyboard_event(event) {
                        handle_keyboard_event(action, &vte, pane_id, &window);
                        return Propagation::Stop;
                    }
                    // Normal button press is handled separately for Tmux
                    window.tmux_keypress(pane_id, key, keyval, state)
                }
                Propagation::Proceed
            }
        ));
        vte.add_controller(eventctl);

        terminal
    }

    pub fn pane_id(&self) -> u32 {
        self.imp().id.get()
    }

    pub fn update_config(
        &self,
        font_desc: &FontDescription,
        main_colors: [RGBA; 2],
        palette_colors: [RGBA; 16],
        scrollback_lines: u32,
    ) {
        let [foreground, background] = main_colors;
        let palette: Vec<&RGBA> = palette_colors.iter().map(|c| c).collect();

        let binding = self.imp().vte.borrow();
        let vte = binding.as_ref().unwrap();
        vte.set_font(Some(&font_desc));
        vte.set_colors(Some(&foreground), Some(&background), &palette[..]);
        vte.set_scrollback_lines(scrollback_lines as i64);
    }

    pub fn feed_output(&self, output: Vec<u8>, initial: bool) {
        let imp = self.imp();

        if initial == false && imp.is_synced() == false {
            // Regular output, but we are NOT yet synced!
            return;
        }

        let binding = self.imp().vte.borrow();
        let vte = binding.as_ref().unwrap();
        vte.feed(&output);
    }

    pub fn initial_output_finished(&self) {
        self.imp().set_synced();
    }

    pub fn scroll_view(&self, empty_lines: usize) {
        if empty_lines < 1 {
            return;
        }

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

        self.feed_output(output, false);
    }

    pub fn get_cols_or_rows(&self) -> (i64, i64) {
        let binding = self.imp().vte.borrow();
        let vte = binding.as_ref().unwrap();

        let cols = vte.column_count();
        let rows = vte.row_count();
        (cols, rows)
    }

    pub fn get_char_width_height(&self) -> (i32, i32) {
        let binding = self.imp().vte.borrow();
        let vte = binding.as_ref().unwrap();
        (vte.char_width() as i32, vte.char_height() as i32)
    }
}

#[inline]
fn handle_keyboard_event(action: KeyboardAction, vte: &Vte, pane_id: u32, window: &IvyTmuxWindow) {
    match action {
        KeyboardAction::CopySelected => {
            vte.emit_copy_clipboard();
        }
        KeyboardAction::PasteClipboard => {
            window.clipboard_paste_event(pane_id);
        }
        KeyboardAction::TabRename => {
            todo!();
        }
        _ => {
            window.tmux_handle_keybinding(action, pane_id);
        }
    }
}
