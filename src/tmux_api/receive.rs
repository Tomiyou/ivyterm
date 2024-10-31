use std::{
    io::{BufRead, BufReader},
    process::ChildStdout,
    str::from_utf8,
};

use async_channel::{Receiver, Sender};
use log::debug;

use crate::{helpers::open_editor, tmux_api::TmuxEvent};

use super::{parse_layout::parse_tmux_layout, TmuxCommand};

/// Parses Tmux output and replaces octal escapes sequences with correct binary
/// characters
#[inline]
fn parse_escaped_output(input: &[u8], prepend_linebreak: bool, empty_lines: usize) -> Vec<u8> {
    let input_len = input.len();
    let mut output = Vec::with_capacity(input_len + (empty_lines * 2) + 3);

    if prepend_linebreak {
        output.push(b'\r');
        output.push(b'\n');
    }

    for _ in 0..empty_lines {
        output.push(b'\r');
        output.push(b'\n');
    }

    let mut i = 0;
    while i < input_len {
        let char = input[i];
        if char == b'\\' {
            // First \ is an escape, meaning there is 100% another char after it
            if input[i + 1] == b'\\' {
                // First \ is followed by another \
                output.push(b'\\');
                i += 2;
                continue;
            }

            // Maybe an escape sequence?
            if i + 3 >= input_len {
                panic!("Found escape character but string too short");
            }

            // This is an escape sequence
            // TODO: This crashes when output is:
            // tomc:~/dev/ivyterm/target/release (master) $ danes je pa tako M-\M-\
            let mut ascii = 0;
            for j in i + 1..i + 4 {
                let num = input[j] - 48;
                ascii *= 8;
                ascii += num;
            }
            output.push(ascii);

            // We also read 3 extra characters after \
            i += 4;
        } else {
            output.push(char);
            i += 1;
        }
    }

    output
}

#[inline]
fn buffer_starts_with(buffer: &Vec<u8>, prefix: &str) -> bool {
    if prefix.len() > buffer.len() {
        return false;
    }

    let buffer = &buffer[..prefix.len()];
    let prefix = prefix.as_bytes();

    buffer == prefix
}

#[inline]
fn receive_event(event_channel: &Sender<TmuxEvent>, event: TmuxEvent) {
    event_channel.send_blocking(event).unwrap();
}

#[inline]
pub fn tmux_read_stdout(
    stdout_stream: ChildStdout,
    event_channel: Sender<TmuxEvent>,
    command_queue: Receiver<TmuxCommand>,
    ssh_target: Option<String>,
) {
    let mut buffer = Vec::with_capacity(65534);
    let mut reader = BufReader::new(stdout_stream);

    // Variable containing Tmux state
    let mut current_command = None;
    let mut is_error = false;
    let mut result_line = 0;
    let mut empty_line_count = 0;

    // TODO: Handle output larger than 65534 bytes
    while let Ok(bytes_read) = reader.read_until(10, &mut buffer) {
        // All output from Tmux is ASCII, except %output which we handle separately
        if bytes_read == 0 {
            continue;
        }

        // Since we read until (and including) '\n', it will always be at the
        // end of the buffer. We should strip it here.
        buffer.pop();
        if buffer.is_empty() {
            empty_line_count += 1;
            continue;
        }

        debug!("Tmux output: {}", from_utf8(&buffer).unwrap());

        if buffer.is_empty() || buffer[0] != b'%' {
            // This is output from a command we ran
            if is_error {
                let error = from_utf8(&buffer).unwrap();
                eprintln!("Error: ({:?}) {}", current_command, error);
            } else if let Some(command) = &current_command {
                tmux_command_result(
                    command,
                    &mut buffer,
                    result_line,
                    empty_line_count,
                    &event_channel,
                    &ssh_target,
                );
            }

            result_line += 1;
            empty_line_count = 0;
        } else {
            // This is a notification
            if buffer_starts_with(&buffer, "%output") {
                // We were given output, we can assume that up until pane_id, output is ASCII
                let (pane_id, chars_read) = read_first_u32(&buffer[9..]);
                let output = parse_escaped_output(&buffer[9 + chars_read..], false, 0);

                receive_event(&event_channel, TmuxEvent::Output(pane_id, output, false));
            } else if buffer_starts_with(&buffer, "%begin") {
                // Beginning of output from a command we executed
                current_command = Some(command_queue.recv_blocking().unwrap());
            } else if buffer_starts_with(&buffer, "%end") {
                // End of output from a command we executed
                if let Some(current_command) = current_command {
                    match current_command {
                        TmuxCommand::InitialOutput(pane_id) => {
                            receive_event(
                                &event_channel,
                                TmuxEvent::ScrollOutput(pane_id, empty_line_count),
                            );
                            receive_event(
                                &event_channel,
                                TmuxEvent::InitialOutputFinished(pane_id),
                            );
                        }
                        TmuxCommand::ChangeSize(_, _) => {
                            receive_event(&event_channel, TmuxEvent::SizeChanged);
                        }
                        TmuxCommand::InitialLayout => {
                            receive_event(&event_channel, TmuxEvent::InitialLayoutFinished);
                        }
                        _ => {}
                    }
                }

                current_command = None;
                is_error = false;
                result_line = 0;
                empty_line_count = 0;
            } else if buffer_starts_with(&buffer, "%error") {
                // TODO: We still don't actually print the error
                eprintln!("Error on command {:?}", current_command);

                // Command we executed produced an error
                current_command = None;
                is_error = false;
                result_line = 0;
                empty_line_count = 0;
            } else if buffer_starts_with(&buffer, "%window-pane-changed") {
                // %window-pane-changed @0 %10
                let (tab_id, chars_read) = read_first_u32(&buffer[22..]);
                let buffer = &buffer[22 + chars_read + 1..];
                let (pane_id, _) = read_first_u32(buffer);
                debug!(
                    "Tmux event: Window {} focus changed to pane {}",
                    tab_id, pane_id
                );
                receive_event(&event_channel, TmuxEvent::PaneFocusChanged(tab_id, pane_id));
            } else if buffer_starts_with(&buffer, "%window-add") {
                // TODO: Instead of asking for info when creating a new window, ask for info
                // after receiving this notification
                // %window-add @32
            } else if buffer_starts_with(&buffer, "%session-window-changed") {
                // %session-window-changed $1 @1
                let (session_id, chars_read) = read_first_u32(&buffer[25..]);
                let buffer = &buffer[25 + chars_read + 1..];
                let (tab_id, _) = read_first_u32(buffer);
                debug!(
                    "Tmux event: Session {} focus changed to window {}",
                    session_id, tab_id
                );
                receive_event(&event_channel, TmuxEvent::TabFocusChanged(tab_id));
            } else if buffer_starts_with(&buffer, "%unlinked-window-close") {
                // %unlinked-window-close @6
                let (tab_id, _) = read_first_u32(&buffer[24..]);
                debug!("Tmux event: Tab {} closed", tab_id);
                receive_event(&event_channel, TmuxEvent::TabClosed(tab_id));
            } else if buffer_starts_with(&buffer, "%layout-change") {
                // Layout has changed
                let layout_sync = parse_tmux_layout(&buffer[15..]);
                receive_event(&event_channel, TmuxEvent::LayoutChanged(layout_sync));
            } else if buffer_starts_with(&buffer, "%session-changed") {
                // Session has changed
                let (id, bytes_read) = read_first_u32(&buffer[18..]);
                let name = from_utf8(&buffer[18 + bytes_read..]).unwrap().to_string();
                debug!("Tmux event: Session changed ({}): {}", id, name);

                receive_event(&event_channel, TmuxEvent::SessionChanged(id, name));
            } else if buffer_starts_with(&buffer, "%window-renamed") {
                // Session has changed
                let (id, bytes_read) = read_first_u32(&buffer[17..]);
                let name = from_utf8(&buffer[17 + bytes_read..]).unwrap().to_string();
                debug!("Tmux event: Tab renamed ({}): {}", id, name);

                receive_event(&event_channel, TmuxEvent::TabRenamed(id, name));
            } else if buffer_starts_with(&buffer, "%exit") {
                // Tmux client has exited
                let reason = from_utf8(&buffer[5..]).unwrap();
                println!("Tmux event: Exit received, reason: {}", reason);
                receive_event(&event_channel, TmuxEvent::Exit);
                // Stop receiving events
                return;
            } else if buffer_starts_with(&buffer, "%client-session-changed") {
            } else {
                // Unsupported notification
                let notification = from_utf8(&buffer).unwrap();
                println!("Tmux event: Unknown notification: {}", notification)
            }
        }

        buffer.clear();
    }
    buffer.clear();
}

#[inline]
fn tmux_command_result(
    command: &TmuxCommand,
    buffer: &mut Vec<u8>,
    result_line: usize,
    empty_lines: usize,
    event_channel: &Sender<TmuxEvent>,
    ssh_target: &Option<String>,
) {
    match command {
        TmuxCommand::TabNew => {
            let layout_sync = parse_tmux_layout(buffer);
            receive_event(&event_channel, TmuxEvent::TabNew(layout_sync));
        }
        TmuxCommand::InitialLayout => {
            let layout_sync = parse_tmux_layout(buffer);
            receive_event(&event_channel, TmuxEvent::InitialLayout(layout_sync));
        }
        TmuxCommand::InitialOutput(pane_id) => {
            let output = parse_escaped_output(&buffer, result_line > 0, empty_lines);

            // let mut escaped = String::with_capacity(output.len() * 2);
            // for c in &output {
            //     if c.is_ascii_control() {
            //         let x = std::ascii::escape_default(*c);
            //         for c in x {
            //             escaped.push(c as char);
            //         }
            //     } else {
            //         escaped.push(*c as char);
            //     }
            // }
            // println!("Tmux initial output for pane {}: |{}|", pane_id, escaped);

            receive_event(&event_channel, TmuxEvent::Output(*pane_id, output, true));
        }
        TmuxCommand::PaneCurrentPath(term_id) => {
            let (pane_id, bytes_read) = read_first_u32(&buffer[7..]);
            // Currently Tmux sends paths of all Terminals in the given Tab, so we need
            // to filter manually
            if pane_id == *term_id {
                let path = from_utf8(&buffer[7 + bytes_read..]).unwrap();
                open_editor(path, ssh_target);
            }
        }
        _ => {}
    }
}

#[inline]
pub fn read_first_u32(buffer: &[u8]) -> (u32, usize) {
    let mut i = 0;
    let mut number: u32 = 0;

    // Read buffer char by char (assuming ASCII) and parse number
    while i < buffer.len() && buffer[i] > 47 && buffer[i] < 58 {
        number *= 10;
        number += (buffer[i] - 48) as u32;
        i += 1;
    }
    (number, i + 1)
}
