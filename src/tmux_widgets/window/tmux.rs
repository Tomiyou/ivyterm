use std::time::Duration;

use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::gdk::{Key, ModifierType};
use libadwaita::{glib, prelude::*};
use log::debug;

use crate::{
    keyboard::keycode_to_arrow_key,
    tmux_api::{LayoutFlags, LayoutSync, TmuxEvent},
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

        // TODO: All keys can be handled using keyval.name(), just exclude Ctrl, Shift, Alt, Tab, etc
        // - if char
        // - if Ctrl, Shift, Alt, Tab, etc
        // - else
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
        } else if keycode >= 67 && keycode < 120 {
            // This is a Function key
            if let Some(name) = keyval.name() {
                let name = name.as_str();
                let name = name.replace('_', "");
                tmux.send_function_key(pane_id, &name);
            }
        }
    }

    pub fn send_clipboard(&self, pane_id: u32, text: &str) {
        let mut binding = self.imp().tmux.borrow_mut();
        if let Some(tmux) = binding.as_mut() {
            tmux.send_quoted_text(pane_id, text);
        }
    }

    fn tmux_sync_size(&self) {
        let imp = self.imp();

        let binding = imp.tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();
        let selected_page = tab_view.selected_page();

        if let Some(selected_page) = selected_page {
            let top_level: TmuxTopLevel = selected_page.child().downcast().unwrap();
            debug!(
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

    fn sync_tmux_layout(&self, layout_sync: LayoutSync) {
        let tab_id = layout_sync.tab_id;
        let flags = layout_sync.flags;

        let top_level = if let Some(top_level) = self.get_top_level(tab_id) {
            debug!("Reusing top Level {}", top_level.tab_id());
            top_level
        } else {
            debug!("Creating new Tab (with new top_level)");
            self.new_tab(tab_id)
        };

        // Sync Tab layout
        top_level.sync_tmux_layout(self, layout_sync);

        // If the Tab is focused, we remember that here
        if flags.contains(LayoutFlags::HasFocus) {
            self.imp().focused_tab.replace(tab_id);
        }
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
                    top_level.select_terminal_event(term_id);
                }
            }
            TmuxEvent::TabFocusChanged(tab_id) => {
                debug!("TabFocusChanged {}", tab_id);

                let old = self.imp().focused_tab.replace(tab_id);
                if old != tab_id {
                    let top_level = self.get_top_level(tab_id);

                    if let Some(top_level) = top_level {
                        let binding = self.imp().tab_view.borrow();
                        let tab_view = binding.as_ref().unwrap();
                        let page = tab_view.page(&top_level);
                        tab_view.set_selected_page(&page);
                    }
                }
            }
            TmuxEvent::TabNew(layout_sync) => {
                debug!("\n---------- New tab ----------");
                self.sync_tmux_layout(layout_sync);
            }
            TmuxEvent::TabClosed(tab_id) => {
                if let Some(top_level) = self.get_top_level(tab_id) {
                    self.close_tab(&top_level);
                }
            }
            TmuxEvent::InitialLayout(layout_sync) => {
                // TODO: Fix Tmux not reporting which Terminal is selected in Initial Layout
                // TODO: Block resize until Tmux layout is parsed (or maybe the other way around?)
                // Also only get initial output when size + layout is OK
                // We can calculate TopLevel size: TotalSize - HeaderBar?

                debug!("\n---------- Initial layout ----------");
                self.sync_tmux_layout(layout_sync);
            }
            TmuxEvent::InitialLayoutFinished => {
                // We have initial layout, meaning we can now calculate cols&rows to sync the
                // Tmux client size
                let current_tab = self.imp().focused_tab.get();
                let top_level = self.get_top_level(current_tab);
                if let Some(top_level) = top_level {
                    let binding = self.imp().tab_view.borrow();
                    let tab_view = binding.as_ref().unwrap();
                    let page = tab_view.page(&top_level);
                    tab_view.set_selected_page(&page);
                }

                imp.init_layout_finished.replace(TmuxInitState::SyncingSize);
            }
            TmuxEvent::InitialOutputFinished(pane_id) => {
                let terminals = imp.terminals.borrow();
                if let Some(pane) = terminals.get(pane_id) {
                    pane.initial_output_finished();
                }
            }
            TmuxEvent::LayoutChanged(layout_sync) => {
                debug!("\n---------- Layout changed ----------");
                self.sync_tmux_layout(layout_sync);
            }
            TmuxEvent::SizeChanged => {
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

    pub fn initial_layout_finished(&self) -> bool {
        self.imp().init_layout_finished.get() == TmuxInitState::Done
    }
}
