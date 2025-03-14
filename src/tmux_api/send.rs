use log::debug;

use crate::{
    helpers::TmuxError,
    keyboard::{Direction, KeyboardAction},
    tmux_api::TmuxCommand,
};

use super::TmuxAPI;

impl TmuxAPI {
    #[inline]
    fn send_event(&self, event: TmuxCommand, cmd: &str) -> Result<(), TmuxError> {
        use std::io::Write;

        const NEWLINE: &[u8] = &[b'\n'];
        // First we put the Command in Event queue
        let command_queue = &self.command_queue;
        command_queue
            .send_blocking(event)
            .map_err(|_| TmuxError::EventChannelClosed)?;

        // Then we write the buffer to the Tmux input stream
        debug!("Sending event: {}", cmd);
        let mut stdin_stream = self.stdin_stream.borrow_mut();
        stdin_stream
            .write_all(cmd.as_bytes())
            .map_err(|_| TmuxError::EventChannelClosed)?;
        stdin_stream
            .write_all(NEWLINE)
            .map_err(|_| TmuxError::EventChannelClosed)?;

        Ok(())
    }

    pub fn get_initial_layout(&self) -> Result<(), TmuxError> {
        debug!("Getting initial layout");
        let cmd = "list-windows -F \"#{window_id} #{window_layout} #{window_visible_layout} #{window_flags} #{window_name}\"";
        self.send_event(TmuxCommand::InitialLayout, cmd)
    }

    pub fn get_initial_output(&self, pane_id: u32) -> Result<(), TmuxError> {
        debug!("Getting initial output of pane {}", pane_id);
        let event = TmuxCommand::InitialOutput(pane_id);
        let cmd = format!("capture-pane -J -p -t %{} -eC -S - -E -", pane_id);
        self.send_event(event, &cmd)
    }

    pub fn change_size(&self, cols: i32, rows: i32) -> Result<(), TmuxError> {
        // If Tmux client size hasn't changed, we don't need to send any update
        if self.window_size.get() == (cols, rows) {
            debug!(
                "Not updating Tmux size to {}x{}, since it did not change",
                cols, rows
            );
            return Ok(());
        }
        self.window_size.replace((cols, rows));

        println!("Resizing Tmux client to {}x{}", cols, rows);
        let event = TmuxCommand::ChangeSize(cols, rows);
        let cmd = format!("refresh-client -C {},{}", cols, rows);
        self.send_event(event, &cmd)
    }

    pub fn send_keypress(
        &self,
        pane_id: u32,
        c: char,
        prefix: String,
        movement: Option<&str>,
    ) -> Result<(), TmuxError> {
        let cmd = if let Some(control) = movement {
            // Navigation keys (left, right, page up, ...)
            format!("send-keys -t %{} {}{}", pane_id, prefix, control)
        } else if c.is_ascii_control() {
            // A control character was just pressed
            let ascii = c as u8;
            // Ignore Ctrl and Shift for Enter
            let prefix = if ascii == 13 { "" } else { prefix.as_str() };
            format!("send-keys -t %{} -- {}\\{:03o}", pane_id, prefix, ascii)
        } else {
            // We send single-quoted keys, but what if we want to send a single quote?
            let quote = if c == '\'' { '"' } else { '\'' };

            // If Ctrl/Shift/Alt was pressed, prefix will not be empty and we need to
            // remove Tmux's -l flag
            let flags = if prefix.is_empty() { "-l" } else { "" };

            format!(
                "send-keys -t %{} {} -- {}{}{}{}",
                pane_id, flags, quote, prefix, c, quote
            )
        };

        debug!("send_keypress: {}", &cmd[..cmd.len() - 1]);
        self.send_event(TmuxCommand::Keypress, &cmd)
    }

    pub fn send_quoted_text(&self, pane_id: u32, text: &str) -> Result<(), TmuxError> {
        // Escape content
        let mut escaped = String::with_capacity(text.len());
        for c in text.chars() {
            // Import write!{} trait here, otherwise it collides with
            // use std::io::Write;
            use std::fmt::Write;

            match c {
                // These characters mess with Tmux
                '\n' | '"' | '\\' | '$' => {
                    let ascii = c as u8;
                    write!(escaped, "\\{:03o}", ascii).unwrap();
                }
                _ => escaped.push(c),
            }
        }

        let cmd = format!("send-keys -l -t %{} -- \"{}\"", pane_id, escaped);
        debug!("send_clipboard: {}", &cmd[..cmd.len() - 1]);
        self.send_event(TmuxCommand::ClipboardPaste, &cmd)
    }

    // TODO: Too many functions for sending text
    pub fn send_function_key(&self, pane_id: u32, text: &str) -> Result<(), TmuxError> {
        let cmd = format!("send-keys -t %{} -- \"{}\"", pane_id, text);

        debug!("send_function_key: {}", &cmd[..cmd.len() - 1]);
        self.send_event(TmuxCommand::Keypress, &cmd)
    }

    pub fn send_keybinding(&self, action: KeyboardAction, pane_id: u32) -> Result<(), TmuxError> {
        let (event, cmd) = match action {
            KeyboardAction::PaneSplit(horizontal) => {
                let event = TmuxCommand::PaneSplit(horizontal);
                let cmd = format!(
                    "split-window {} -t %{}",
                    if horizontal { "-v" } else { "-h" },
                    pane_id,
                );
                (event, cmd)
            }
            KeyboardAction::PaneClose => {
                let event = TmuxCommand::PaneClose(pane_id);
                let cmd = format!("kill-pane -t %{}", pane_id);
                (event, cmd)
            }
            KeyboardAction::TabNew => {
                // TODO: We should get all required layout info without having to ask directly,
                // since it would allow us to react to external commands
                let cmd = String::from(
                    "new-window -P -F \"#{window_id} #{window_layout} #{window_visible_layout} ${window_flags} #{window_name}\"",
                );
                (TmuxCommand::TabNew, cmd)
            }
            KeyboardAction::TabClose => {
                let cmd = String::from("kill-window");
                (TmuxCommand::TabClose, cmd)
            }
            KeyboardAction::TabRename => {
                // We do nothing, since Tab renaming is handled separately
                return Ok(());
            }
            KeyboardAction::MoveFocus(direction) => {
                let cmd = format!(
                    "select-pane {}",
                    match direction {
                        Direction::Down => "-D",
                        Direction::Left => "-L",
                        Direction::Right => "-R",
                        Direction::Up => "-U",
                    }
                );
                let event = TmuxCommand::PaneMoveFocus(direction);
                (event, cmd)
            }
            KeyboardAction::ToggleZoom => {
                let cmd = format!("resize-pane -Z -t %{}", pane_id);
                let event = TmuxCommand::PaneZoom(pane_id);
                (event, cmd)
            }
            KeyboardAction::CopySelected => {
                todo!();
            }
            KeyboardAction::PasteClipboard => {
                panic!("PasteClipboard keyboard event needs to be handled by Terminal widget");
            }
            KeyboardAction::OpenEditorCwd => {
                // TODO: This prints ALL panes in a Tab, not needed
                let event = TmuxCommand::PaneCurrentPath(pane_id);
                let cmd = format!(
                    "list-panes -t %{} -F \"path: #{{pane_id}} #{{pane_current_path}}\"",
                    pane_id
                );
                (event, cmd)
            }
            KeyboardAction::ClearScrollback => {
                let event = TmuxCommand::ClearScrollback(pane_id);
                let cmd = format!("clear-history -t %{}", pane_id);
                (event, cmd)
            }
        };

        self.send_event(event, &cmd)
    }

    pub fn select_tab(&self, tab_id: u32) -> Result<(), TmuxError> {
        let event = TmuxCommand::TabSelect(tab_id);
        let cmd = format!("select-window -t @{}", tab_id);
        self.send_event(event, &cmd)
    }

    pub fn select_terminal(&self, term_id: u32) -> Result<(), TmuxError> {
        let event = TmuxCommand::PaneSelect(term_id);
        let cmd = format!("select-pane -t %{}", term_id);
        self.send_event(event, &cmd)
    }

    /// Updates resize_future to `new` value, while returning the old value
    pub fn update_resize_future(&self, new: bool) -> bool {
        self.resize_future.replace(new)
    }

    pub fn rename_tab(&self, tab_id: u32, name: String) -> Result<(), TmuxError> {
        // Handle newlines and escape " character
        let name = name.replace('"', "\\\"");
        let name = if let Some(newline) = name.find('\n') {
            &name[..newline]
        } else {
            &name
        };

        let event = TmuxCommand::TabRename(tab_id);
        let cmd = format!("rename-window -t @{} -- \"{}\"", tab_id, name);
        self.send_event(event, &cmd)
    }

    pub fn resize_pane(
        &self,
        term_id: u32,
        direction: Direction,
        amount: u32,
    ) -> Result<(), TmuxError> {
        let event = TmuxCommand::PaneResize(term_id);
        let direction = match direction {
            Direction::Down => "D",
            Direction::Left => "L",
            Direction::Up => "U",
            Direction::Right => "R",
        };
        let cmd = format!("resize-pane -{} -t %{} {}", direction, term_id, amount);
        self.send_event(event, &cmd)
    }
}
