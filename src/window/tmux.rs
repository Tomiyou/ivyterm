use std::time::Duration;

use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::gdk::{Key, ModifierType};
use libadwaita::{glib, prelude::*};
use log::debug;

use crate::{
    keyboard::keycode_to_arrow_key,
    tmux::{Tmux, TmuxEvent, TmuxTristate},
    toplevel::TopLevel,
    window::layout::parse_tmux_layout,
};

use super::IvyWindow;

const RESIZE_TIMEOUT: Duration = Duration::from_millis(5);

impl IvyWindow {
    pub fn init_tmux(&self, tmux: Tmux) {
        let imp = self.imp();

        // First store Tmux
        imp.tmux.replace(Some(tmux));

        // Then get initial layout - this order to prevent a possible race condition
        let binding = imp.tmux.borrow();
        let tmux = binding.as_ref().unwrap();
        tmux.get_initial_layout();
    }

    pub fn is_tmux(&self) -> bool {
        let binding = self.imp().tmux.borrow();
        binding.is_some()
    }

    pub fn tmux_keypress(&self, pane_id: u32, keycode: u32, keyval: Key, state: ModifierType) {
        let binding = self.imp().tmux.borrow();
        let tmux = binding.as_ref().unwrap();

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
            let top_level: TopLevel = selected_page.child().downcast().unwrap();
            let (cols, rows) = top_level.get_cols_rows();

            let mut binding = self.imp().tmux.borrow_mut();
            let tmux = binding.as_mut().unwrap();
            // Tell Tmux resize future is no longer running
            tmux.update_resize_future(false);
            tmux.change_size(cols as i32, rows as i32);
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

    pub fn tmux_event_callback(&self, event: TmuxEvent) {
        let imp = self.imp();

        // This future runs on main thread of GTK application
        // It receives Tmux events from separate thread and runs GTK functions
        match event {
            TmuxEvent::Output(pane_id, output) => {
                // Ignore Output events until initial output has been captured
                let tmux = imp.tmux.borrow();
                if tmux.as_ref().unwrap().initial_output == TmuxTristate::Uninitialized {
                    return;
                }

                let terminals = imp.terminals.borrow();
                if let Some(pane) = terminals.get(&pane_id) {
                    pane.feed_output(output);
                }
            }
            TmuxEvent::InitialLayout(layout) => {
                println!("Given layout: {}", std::str::from_utf8(&layout).unwrap());
                parse_tmux_layout(&layout[1..], &self);

                // TODO: Block resize until Tmux layout is parsed (or maybe the other way around?)
                // Also only get initial output when size + layout is OK
            }
            TmuxEvent::InitialOutputFinished() => {
                let mut binding = imp.tmux.borrow_mut();
                let tmux = binding.as_mut().unwrap();
                tmux.initial_output = TmuxTristate::Done;
            }
            TmuxEvent::LayoutChanged(layout) => {
                // todo!()
            }
            TmuxEvent::SizeChanged() => {
                let mut binding = imp.tmux.borrow_mut();
                let tmux = binding.as_mut().unwrap();

                // If initial output has not been captured yet, now is the time
                if tmux.initial_output == TmuxTristate::Uninitialized {
                    println!("Getting initial output");
                    tmux.initial_output = TmuxTristate::WaitingResponse;

                    let terminals = imp.terminals.borrow();
                    for (pane_id, _) in terminals.iter() {
                        tmux.get_initial_output(*pane_id);
                    }
                }
            }
            TmuxEvent::Exit => {
                println!("Received EXIT event, closing window!");
                self.close();
            }
            TmuxEvent::ScrollOutput(pane_id, empty_lines) => {
                let binding = &self.imp().terminals;
                if let Some(pane) = binding.borrow().get(&pane_id) {
                    pane.scroll_view(empty_lines);
                }
            }
        }
    }

    pub fn tmux_layout_callback() {}
}
