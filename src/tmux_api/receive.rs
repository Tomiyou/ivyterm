use std::{
    io::{BufRead, BufReader},
    process::ChildStdout,
    str::from_utf8,
};

use async_channel::{Receiver, Sender};
use log::debug;

use crate::tmux_api::TmuxEvent;

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
pub fn tmux_read_stdout(
    stdout_stream: ChildStdout,
    event_channel: Sender<TmuxEvent>,
    command_queue: Receiver<TmuxCommand>,
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
                println!("Error: ({:?}) {}", current_command, error);
            } else if let Some(command) = &current_command {
                tmux_command_result(
                    command,
                    &mut buffer,
                    result_line,
                    empty_line_count,
                    &event_channel,
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

                event_channel
                    .send_blocking(TmuxEvent::Output(pane_id, output, false))
                    .expect("Event channel closed!");
            } else if buffer_starts_with(&buffer, "%begin") {
                // Beginning of output from a command we executed
                current_command = Some(command_queue.recv_blocking().unwrap());
            } else if buffer_starts_with(&buffer, "%end") {
                // End of output from a command we executed
                if let Some(current_command) = current_command {
                    match current_command {
                        TmuxCommand::InitialOutput(pane_id) => {
                            event_channel
                                .send_blocking(TmuxEvent::ScrollOutput(pane_id, empty_line_count))
                                .expect("Event channel closed!");
                            event_channel
                                .send_blocking(TmuxEvent::InitialOutputFinished(pane_id))
                                .expect("Event channel closed!");
                        }
                        TmuxCommand::ChangeSize(_, _) => {
                            event_channel
                                .send_blocking(TmuxEvent::SizeChanged)
                                .expect("Event channel closed!");
                        }
                        TmuxCommand::InitialLayout => {
                            event_channel
                                .send_blocking(TmuxEvent::InitialLayoutFinished)
                                .expect("Event channel closed!");
                        }
                        _ => {}
                    }
                }

                current_command = None;
                is_error = false;
                result_line = 0;
                empty_line_count = 0;
            } else if buffer_starts_with(&buffer, "%window-pane-changed") {
                // %window-pane-changed @0 %10
                let (tab_id, chars_read) = read_first_u32(&buffer[22..]);
                let buffer = &buffer[22 + chars_read + 1..];
                let (pane_id, _) = read_first_u32(buffer);
                println!(
                    "Tmux event: Window {} focus changed to pane {}",
                    tab_id, pane_id
                );
                event_channel
                    .send_blocking(TmuxEvent::PaneFocusChanged(tab_id, pane_id))
                    .unwrap();
            } else if buffer_starts_with(&buffer, "%session-window-changed") {
                // %session-window-changed $1 @1
                let (session_id, chars_read) = read_first_u32(&buffer[25..]);
                let buffer = &buffer[25 + chars_read + 1..];
                let (tab_id, _) = read_first_u32(buffer);
                println!(
                    "Tmux event: Session {} focus changed to window {}",
                    session_id, tab_id
                );
                event_channel
                    .send_blocking(TmuxEvent::TabFocusChanged(tab_id))
                    .unwrap();
            } else if buffer_starts_with(&buffer, "%unlinked-window-close") {
                // %unlinked-window-close @6
                let (tab_id, _) = read_first_u32(&buffer[24..]);
                println!("Tmux event: Tab {} closed", tab_id);
                event_channel
                    .send_blocking(TmuxEvent::TabClosed(tab_id))
                    .unwrap();
            } else if buffer_starts_with(&buffer, "%layout-change") {
                // Layout has changed
                let layout_sync = parse_tmux_layout(&buffer[15..]);
                event_channel
                    .send_blocking(TmuxEvent::LayoutChanged(layout_sync))
                    .unwrap();
            } else if buffer_starts_with(&buffer, "%error") {
                // Command we executed produced an error
                current_command = Some(command_queue.recv_blocking().unwrap());
                is_error = true;
            } else if buffer_starts_with(&buffer, "%session-changed") {
                // Session has changed
                let (id, bytes_read) = read_first_u32(&buffer[18..]);
                let name = from_utf8(&buffer[18 + bytes_read..]).unwrap().to_string();
                debug!("Tmux event: Session changed ({}): {}", id, name);

                event_channel
                    .send_blocking(TmuxEvent::SessionChanged(id, name))
                    .unwrap();
            } else if buffer_starts_with(&buffer, "%exit") {
                // Tmux client has exited
                let reason = from_utf8(&buffer[5..]).unwrap();
                println!("Tmux event: Exit received, reason: {}", reason);
                event_channel
                    .send_blocking(TmuxEvent::Exit)
                    .expect("Event channel closed!");
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
) {
    match command {
        TmuxCommand::TabNew => {
            let layout_sync = parse_tmux_layout(buffer);
            event_channel
                .send_blocking(TmuxEvent::TabNew(layout_sync))
                .unwrap();
        }
        TmuxCommand::InitialLayout => {
            let layout_sync = parse_tmux_layout(buffer);
            event_channel
                .send_blocking(TmuxEvent::InitialLayout(layout_sync))
                .unwrap();
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

            event_channel
                .send_blocking(TmuxEvent::Output(*pane_id, output, true))
                .expect("Event channel closed!");
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
