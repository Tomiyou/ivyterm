use std::io::{BufRead, BufReader, Write};

use log::debug;

use crate::{keyboard::KeyboardAction, tmux_api::TmuxCommand};

use super::TmuxAPI;

impl TmuxAPI {
    pub fn get_initial_layout(&self) {
        let command_queue = &self.command_queue;
        let mut stdin_stream = &self.stdin_stream;

        let command = TmuxCommand::InitialLayout;

        debug!("Getting initial layout");
        command_queue.send_blocking(command).unwrap();
        stdin_stream
            .write_all(b"list-windows -F \"#{window_id},#{window_layout}\"\n")
            .unwrap();
    }

    pub fn get_initial_output(&self, pane_id: u32) {
        let command_queue = &self.command_queue;
        let mut stdin_stream = &self.stdin_stream;

        debug!("Getting initial output of pane {}", pane_id);
        let cmd = format!("capture-pane -J -p -t %{} -eC -S - -E -\n", pane_id);
        command_queue
            .send_blocking(TmuxCommand::InitialOutput(pane_id))
            .unwrap();
        stdin_stream.write_all(cmd.as_bytes()).unwrap();
    }

    pub fn change_size(&mut self, cols: i32, rows: i32) {
        if self.window_size == (cols, rows) {
            println!(
                "Not updating Tmux size to {}x{}, since it did not change",
                cols, rows
            );
            return;
        }
        self.window_size = (cols, rows);

        let command = TmuxCommand::ChangeSize(cols, rows);

        println!("Resizing Tmux client to {}x{}", cols, rows);
        let cmd = format!("refresh-client -C {},{}\n", cols, rows);
        self.command_queue.send_blocking(command).unwrap();
        self.stdin_stream.write_all(cmd.as_bytes()).unwrap();
    }

    pub fn send_keypress(&self, pane_id: u32, c: char, prefix: String, movement: Option<&str>) {
        let command_queue = &self.command_queue;
        let mut stdin_stream = &self.stdin_stream;

        let cmd = if let Some(control) = movement {
            // Navigation keys (left, right, page up, ...)
            format!("send-keys -t %{} {}{}\n", pane_id, prefix, control)
        } else if c.is_ascii_control() {
            // A control character was just pressed
            let ascii = c as u8;
            format!("send-keys -t %{} -- {}\\{:03o}\n", pane_id, prefix, ascii)
        } else {
            // We send single-quoted keys, but what if we want to send a single quote?
            let quote = if c == '\'' { '"' } else { '\'' };

            // If Ctrl/Shift/Alt was pressed, prefix will not be empty and we need to
            // remove Tmux's -l flag
            let flags = if prefix.is_empty() { "-l" } else { "" };

            format!(
                "send-keys -t %{} {} -- {}{}{}{}\n",
                pane_id, flags, quote, prefix, c, quote
            )
        };

        debug!("send_keypress: {}", &cmd[..cmd.len() - 1]);
        command_queue.send_blocking(TmuxCommand::Keypress).unwrap();
        stdin_stream.write_all(cmd.as_bytes()).unwrap();
    }

    pub fn send_keybinding(&self, action: KeyboardAction, pane_id: u32) {
        let command_queue = &self.command_queue;
        let mut stdin_stream = &self.stdin_stream;

        let cmd = match action {
            KeyboardAction::PaneSplit(horizontal) => {
                command_queue
                    .send_blocking(TmuxCommand::PaneSplit(horizontal))
                    .unwrap();
                format!(
                    "split-window {} -t %{} -P -F \"#{{window_id}},#{{window_layout}}\"\n",
                    if horizontal { "-v" } else { "-h" },
                    pane_id,
                )
            }
            KeyboardAction::PaneClose => {
                // top_level.close_pane(terminal);
                todo!();
            }
            KeyboardAction::TabNew => {
                // top_level.create_tab(None);
                todo!();
            }
            KeyboardAction::TabClose => {
                // top_level.close_tab();
                todo!();
            }
            KeyboardAction::SelectPane(direction) => {
                todo!();
                // let previous_size = top_level.unzoom();
                // if let Some(new_focus) = top_level.find_neighbor(terminal, direction, previous_size)
                // {
                //     new_focus.grab_focus();
                // }
            }
            KeyboardAction::ToggleZoom => {
                todo!();
                // top_level.toggle_zoom(terminal);
            }
            KeyboardAction::CopySelected => {
                todo!();
                // vte.emit_copy_clipboard();
            }
        };

        stdin_stream.write_all(cmd.as_bytes()).unwrap();
    }

    /// Updates resize_future to `new` value, while returning the old value
    pub fn update_resize_future(&mut self, new: bool) -> bool {
        let old = self.resize_future;
        self.resize_future = new;
        return old;
    }
}
