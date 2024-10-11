use std::io::Write;

use log::debug;

use crate::{
    keyboard::{Direction, KeyboardAction},
    tmux_api::TmuxCommand,
};

use super::TmuxAPI;

impl TmuxAPI {
    pub fn get_initial_layout(&self) {
        let command_queue = &self.command_queue;
        let mut stdin_stream = &self.stdin_stream;

        let command = TmuxCommand::InitialLayout;

        debug!("Getting initial layout");
        command_queue.send_blocking(command).unwrap();
        stdin_stream
            .write_all(
                b"list-windows -F \"#{window_id} #{window_layout} #{window_visible_layout} #{window_flags}\"\n",
            )
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
                    "split-window {} -t %{} -P -F \"#{{window_id}} #{{window_layout}} #{{window_visible_layout}} #{{window_flags}}\"\n",
                    if horizontal { "-v" } else { "-h" },
                    pane_id,
                )
            }
            KeyboardAction::PaneClose => {
                command_queue
                    .send_blocking(TmuxCommand::PaneClose(pane_id))
                    .unwrap();
                format!("kill-pane -t {}\n", pane_id)
            }
            KeyboardAction::TabNew => {
                command_queue.send_blocking(TmuxCommand::TabNew).unwrap();
                // TODO: We should get all required layout info without having to ask directly,
                // since it would allow us to react to external commands
                String::from(
                    "new-window -P -F \"#{window_id} #{window_layout} #{window_visible_layout} ${window_flags}\"\n",
                )
            }
            KeyboardAction::TabClose => {
                command_queue.send_blocking(TmuxCommand::TabClose).unwrap();
                String::from("kill-window\n")
            }
            KeyboardAction::TabRename => {
                // We do nothing, since Tab renaming is handled separately
                return;
            }
            KeyboardAction::SelectPane(direction) => {
                let cmd = format!(
                    "select-pane {}\n",
                    match direction {
                        Direction::Down => "-D",
                        Direction::Left => "-L",
                        Direction::Right => "-R",
                        Direction::Up => "-U",
                    }
                );
                command_queue
                    .send_blocking(TmuxCommand::PaneSelect(direction))
                    .unwrap();
                cmd
            }
            KeyboardAction::ToggleZoom => {
                let cmd = format!("resize-pane -Z -t {}\n", pane_id);
                command_queue
                    .send_blocking(TmuxCommand::PaneZoom(pane_id))
                    .unwrap();
                cmd
            }
            KeyboardAction::CopySelected => {
                todo!();
            }
        };

        stdin_stream.write_all(cmd.as_bytes()).unwrap();
    }

    pub fn select_tab(&self, tab_id: u32) {
        let command_queue = &self.command_queue;
        let mut stdin_stream = &self.stdin_stream;

        command_queue
            .send_blocking(TmuxCommand::TabSelect(tab_id))
            .unwrap();

        let cmd = format!("select-window -t @{}\n", tab_id);
        stdin_stream.write_all(cmd.as_bytes()).unwrap();
    }

    /// Updates resize_future to `new` value, while returning the old value
    pub fn update_resize_future(&mut self, new: bool) -> bool {
        let old = self.resize_future;
        self.resize_future = new;
        return old;
    }
}
