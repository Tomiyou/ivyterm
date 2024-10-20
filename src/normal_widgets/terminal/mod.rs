mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Propagation, SpawnFlags};
use gtk4::{gdk::RGBA, pango::FontDescription, EventControllerKey, Orientation, ScrolledWindow};
use libadwaita::{glib, prelude::*};
use vte4::{PtyFlags, Terminal as Vte, TerminalExt, TerminalExtManual};

use crate::{application::IvyApplication, keyboard::KeyboardAction};

use super::{toplevel::TopLevel, window::IvyNormalWindow};

glib::wrapper! {
    pub struct Terminal(ObjectSubclass<imp::TerminalPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Terminal {
    pub fn new(top_level: &TopLevel, window: &IvyNormalWindow, pane_id: Option<u32>) -> Self {
        let window = window.clone();

        let pane_id = match pane_id {
            Some(pane_id) => pane_id,
            None => window.unique_terminal_id(),
        };

        let top_level = top_level.clone();

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

        // Add terminal to top level terminal list
        top_level.register_terminal(&terminal);

        // Close terminal + pane/tab when the child (shell) exits
        vte.connect_child_exited(glib::clone!(
            #[weak]
            top_level,
            #[weak]
            terminal,
            move |_, _| {
                top_level.close_pane(&terminal);
            }
        ));

        // Set terminal colors
        let palette: Vec<&RGBA> = palette.iter().map(|c| c).collect();
        vte.set_colors(Some(&foreground), Some(&background), &palette[..]);

        vte.connect_has_focus_notify(glib::clone!(
            #[weak]
            top_level,
            #[weak]
            terminal,
            move |vte| {
                if vte.has_focus() {
                    // Notify TopLevel that the focused terminal changed
                    top_level.focus_changed(pane_id, &terminal);
                }
            }
        ));

        let eventctl = EventControllerKey::new();
        eventctl.connect_key_pressed(glib::clone!(
            #[strong]
            terminal,
            #[strong]
            vte,
            move |eventctl, _keyval, _key, _state| {
                if let Some(event) = eventctl.current_event() {
                    // Check if pressed keys match a keybinding
                    if let Some(action) = app.handle_keyboard_event(event) {
                        handle_keyboard(action, &terminal, &top_level, &vte);
                        return Propagation::Stop;
                    }
                }
                Propagation::Proceed
            }
        ));
        vte.add_controller(eventctl);

        // Spawn terminal
        let pty_flags = PtyFlags::DEFAULT;
        let argv = ["/bin/bash"];
        let envv = [];
        let spawn_flags = SpawnFlags::DEFAULT;

        vte.spawn_async(
            pty_flags,
            None,
            &argv,
            &envv,
            spawn_flags,
            || {},
            -1,
            gtk4::gio::Cancellable::NONE,
            glib::clone!(
                #[weak]
                vte,
                move |_result| {
                    vte.grab_focus();
                }
            ),
        );

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
}

#[inline]
fn handle_keyboard(action: KeyboardAction, terminal: &Terminal, top_level: &TopLevel, vte: &Vte) {
    match action {
        KeyboardAction::PaneSplit(vertical) => {
            let orientation = if vertical {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            };

            top_level.split_pane(terminal, orientation);
        }
        KeyboardAction::PaneClose => {
            top_level.close_pane(terminal);
        }
        KeyboardAction::TabNew => {
            top_level.create_tab();
        }
        KeyboardAction::TabClose => {
            top_level.close_tab();
        }
        KeyboardAction::MoveFocus(direction) => {
            let previous_size = top_level.unzoom();
            if let Some(new_focus) = top_level.find_neighbor(terminal, direction, previous_size) {
                new_focus.grab_focus();
            }
        }
        KeyboardAction::ToggleZoom => {
            top_level.toggle_zoom(terminal);
        }
        KeyboardAction::CopySelected => {
            vte.emit_copy_clipboard();
        }
        KeyboardAction::TabRename => {
            top_level.open_rename_modal();
        }
        KeyboardAction::PasteClipboard => {
            vte.paste_clipboard();
        }
    }
}
