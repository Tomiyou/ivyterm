use std::time::Duration;

use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::gdk::{Key, ModifierType};
use libadwaita::{glib, prelude::*};
use log::debug;

use crate::{
    keyboard::{keycode_to_arrow_key, KeyboardAction},
    tmux_api::{TmuxEvent, TmuxPane},
    tmux_widgets::toplevel::TmuxTopLevel,
};

use super::IvyTmuxWindow;

const RESIZE_TIMEOUT: Duration = Duration::from_millis(5);

#[derive(Clone, Copy, PartialEq)]
pub enum TmuxInitState {
    SyncingLayout,
    SyncingSize,
    Done,
}

impl Default for TmuxInitState {
    fn default() -> Self {
        TmuxInitState::SyncingLayout
    }
}

// Tmux session initialization:
// 1. TmuxWindow constructor calls tmux.get_initial_layout()
// 2. We receive initial layout, which is used to construct the hierarchy
// 3. TopLevel layout.alloc_changed() triggers, which sends Tmux size sync event
// 4. After we receive Tmux size sync conformation, we start getting initial output

impl IvyTmuxWindow {
    pub fn get_char_size(&self) -> (i32, i32) {
        self.imp().char_size.get()
    }

    pub fn tmux_keypress(&self, pane_id: u32, keycode: u32, keyval: Key, state: ModifierType) {
        let binding = self.imp().tmux.borrow();
        let tmux = match binding.as_ref() {
            Some(tmux) => tmux,
            None => return,
        };

        let mut prefix = String::new();
        let mut shift_relevant = false;
        if state.contains(ModifierType::ALT_MASK) {
            prefix.push_str("M-");
            shift_relevant = true;

            // Hacky workaround for Alt+Backspace
            if keycode == 22 {
                tmux.send_keypress(pane_id, '\x7f', prefix, None);
                return;
            }
        }
        if state.contains(ModifierType::CONTROL_MASK) {
            prefix.push_str("C-");
            shift_relevant = true;
        }
        // Uppercase characters work without S-, so this case is only
        // relevant when Ctrl/Alt is also pressed
        if state.contains(ModifierType::SHIFT_MASK) && shift_relevant {
            prefix.push_str("S-");
        }

        if let Some(c) = keyval.to_unicode() {
            tmux.send_keypress(pane_id, c, prefix, None);
        } else if let Some(direction) = keycode_to_arrow_key(keycode) {
            let direction = match direction {
                crate::keyboard::Direction::Left => "Left",
                crate::keyboard::Direction::Right => "Right",
                crate::keyboard::Direction::Up => "Up",
                crate::keyboard::Direction::Down => "Down",
            };
            tmux.send_keypress(pane_id, ' ', prefix, Some(direction));
        }
    }

    fn tmux_sync_size(&self) {
        let imp = self.imp();

        let binding = imp.tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();
        let selected_page = tab_view.selected_page();

        if let Some(selected_page) = selected_page {
            let top_level: TmuxTopLevel = selected_page.child().downcast().unwrap();
            println!(
                "Top Level width {} height {}",
                top_level.width(),
                top_level.height()
            );
            let (cols, rows) = top_level.get_cols_rows();

            let mut binding = self.imp().tmux.borrow_mut();
            if let Some(tmux) = binding.as_mut() {
                // Tell Tmux resize future is no longer running
                tmux.update_resize_future(false);
                tmux.change_size(cols, rows);
            }
        }
    }

    pub fn resync_tmux_size(&self) {
        // First check if a future is already running
        let mut binding = self.imp().tmux.borrow_mut();
        if let Some(tmux) = binding.as_mut() {
            if tmux.update_resize_future(true) {
                // A future is already running, we can stop
                return;
            }

            let window = self.clone();
            glib::spawn_future_local(async move {
                glib::timeout_future(RESIZE_TIMEOUT).await;
                window.tmux_sync_size();
            });
        }
    }

    fn sync_tmux_layout(&self, tab_id: u32, layout: Vec<TmuxPane>, visible_layout: Vec<TmuxPane>) {
        let top_level = if let Some(top_level) = self.get_top_level(tab_id) {
            debug!("Reusing top Level {}", top_level.tab_id());
            top_level
        } else {
            debug!("Creating new Tab (with new top_level)");
            self.new_tab(tab_id)
        };

        top_level.sync_tmux_layout(self, layout);
    }

    pub fn tmux_event_callback(&self, event: TmuxEvent) {
        let imp = self.imp();

        // If Tmux API is finished, we are not doing anything
        if imp.tmux.borrow().is_none() {
            return;
        }

        // This future runs on main thread of GTK application
        // It receives Tmux events from separate thread and runs GTK functions
        match event {
            TmuxEvent::Output(pane_id, output, initial) => {
                // Ignore Output events until initial output has been captured
                let terminals = imp.terminals.borrow();
                if let Some(pane) = terminals.get(pane_id) {
                    pane.feed_output(output, initial);
                }
            }
            TmuxEvent::PaneFocusChanged(tab_id, term_id) => {
                if let Some(top_level) = self.get_top_level(tab_id) {
                    top_level.focus_changed(term_id);
                }
            }
            TmuxEvent::TabFocusChanged(tab_id) => {
                self.imp().focused_tab.replace(tab_id);

                if let Some(top_level) = self.get_top_level(tab_id) {
                    top_level.grab_focus();
                    top_level.focus_current_terminal();
                }
            }
            TmuxEvent::TabNew(tab_id, layout, visible_layout) => {
                println!("\n---------- New tab ----------");
                self.sync_tmux_layout(tab_id, layout, visible_layout);
            }
            TmuxEvent::TabClosed(tab_id) => {
                if let Some(top_level) = self.get_top_level(tab_id) {
                    self.close_tab(&top_level);
                }
            }
            TmuxEvent::InitialLayout(tab_id, layout, visible_layout) => {
                // TODO: Block resize until Tmux layout is parsed (or maybe the other way around?)
                // Also only get initial output when size + layout is OK
                // We can calculate TopLevel size: TotalSize - HeaderBar?

                println!("\n---------- Initial layout ----------");
                self.sync_tmux_layout(tab_id, layout, visible_layout);
                if let Some(top_level) = self.get_top_level(tab_id) {
                    top_level.set_initialized();
                }

                // We have initial layout, meaning we can now calculate cols&rows to sync the
                // Tmux client size
                imp.init_layout_finished.replace(TmuxInitState::SyncingSize);
            }
            TmuxEvent::InitialOutputFinished(pane_id) => {
                let terminals = imp.terminals.borrow();
                if let Some(pane) = terminals.get(pane_id) {
                    pane.initial_output_finished();
                }
            }
            TmuxEvent::LayoutChanged(tab_id, layout, visible_layout) => {
                println!("\n---------- Layout changed ----------");
                self.sync_tmux_layout(tab_id, layout, visible_layout);
            }
            TmuxEvent::SizeChanged() => {
                if imp.init_layout_finished.get() == TmuxInitState::SyncingSize {
                    imp.init_layout_finished.replace(TmuxInitState::Done);

                    let mut binding = imp.tmux.borrow_mut();
                    if let Some(tmux) = binding.as_mut() {
                        // If initial output has not been captured yet, now is the time
                        println!("Getting initial output");
                        let terminals = imp.terminals.borrow();
                        for sorted in terminals.iter() {
                            tmux.get_initial_output(sorted.id);
                        }
                    }
                }
            }
            TmuxEvent::Exit => {
                println!("Received EXIT event, closing window!");
                self.close_tmux_window();
            }
            TmuxEvent::ScrollOutput(pane_id, empty_lines) => {
                let binding = &self.imp().terminals;
                if let Some(pane) = binding.borrow().get(pane_id) {
                    pane.scroll_view(empty_lines);
                }
            }
            TmuxEvent::SessionChanged(id, name) => {
                let new = (id, name.clone());
                let old = imp.session.replace(Some((id, name)));

                // If session changes (after it was already initialized), then
                // something went wrong
                if let Some(old) = old {
                    if old != new {
                        println!("Session {} changed underneath us, closing Window", old.1);
                        self.close_tmux_window();
                    }
                }

                println!("Session {} with name {} initialized", new.0, new.1);
            }
        }
    }

    #[inline]
    pub fn tmux_handle_keybinding(&self, action: KeyboardAction, pane_id: u32) {
        let tmux = self.imp().tmux.borrow();
        if let Some(tmux) = tmux.as_ref() {
            tmux.send_keybinding(action, pane_id);
        }
    }

    pub fn initial_output_finished(&self) -> bool {
        self.imp().init_layout_finished.get() == TmuxInitState::Done
    }
}
