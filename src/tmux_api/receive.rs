use std::{io::BufRead, str::from_utf8};

use async_channel::Sender;
use log::debug;
use vmap::io::Ring;

use crate::{
    helpers::{open_editor, TmuxError},
    tmux_api::TmuxEvent,
};

use super::{parse_layout::parse_tmux_layout, TmuxCommand, TmuxParserState};

pub fn tmux_parse_data(
    state: &mut TmuxParserState,
    ring_buffer: &mut Ring,
) -> Result<(), TmuxError> {
    let mut consumed_bytes = 0;
    let buffer = ring_buffer.as_ref();

    for (i, b) in buffer.iter().enumerate() {
        if *b == b'\n' {
            let buffer = &buffer[consumed_bytes..i];
            consumed_bytes += tmux_parse_line(state, buffer)? + 1; // +1 to account for \n
        }
    }

    // Move the ringbuffer read position
    ring_buffer.consume(consumed_bytes);

    Ok(())
}

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
fn buffer_starts_with(buffer: &[u8], prefix: &str) -> bool {
    if prefix.len() > buffer.len() {
        return false;
    }

    let buffer = &buffer[..prefix.len()];
    let prefix = prefix.as_bytes();

    buffer == prefix
}

#[inline]
fn receive_event(event_channel: &Sender<TmuxEvent>, event: TmuxEvent) -> Result<(), TmuxError> {
    event_channel
        .send_blocking(event)
        .map_err(|_| TmuxError::EventChannelClosed)
}

#[inline]
fn parse_utf8(buffer: &[u8]) -> Result<&str, TmuxError> {
    from_utf8(buffer).map_err(|_| TmuxError::ErrorParsingUTF8)
}

#[inline]
pub fn tmux_parse_line(state: &mut TmuxParserState, buffer: &[u8]) -> Result<usize, TmuxError> {
    let event_channel = &mut state.event_channel;
    let command_queue = &mut state.command_queue;

    // TODO: Handle output larger than 65534 bytes
    // All output from Tmux is ASCII, except %output which we handle separately
    if buffer.len() == 0 {
        return Ok(0);
    }

    debug!("Tmux output: .{}.", parse_utf8(&buffer)?);

    // If buffer is empty or does not start with %, then this is output from a
    // command we executed
    if buffer.is_empty() || buffer[0] != b'%' {
        // This is output from a command we ran
        if state.is_error {
            // TODO: Write to stderr directly
            let error = parse_utf8(&buffer)?;
            eprintln!("Error: ({:?}) {}", state.current_command, error);
        } else if let Some(command) = &state.current_command {
            tmux_command_result(
                command,
                buffer,
                state.result_line,
                state.empty_line_count,
                &event_channel,
                &state.ssh_target,
            )?;
        }

        state.result_line += 1;
        state.empty_line_count = 0;

        return Ok(buffer.len());
    }

    // This is a notification
    if buffer_starts_with(&buffer, "%output") {
        // We were given output, we can assume that up until pane_id, output is ASCII
        let (pane_id, chars_read) = read_first_u32(&buffer[9..]);
        let output = parse_escaped_output(&buffer[9 + chars_read..], false, 0);

        receive_event(&event_channel, TmuxEvent::Output(pane_id, output, false))?;
    } else if buffer_starts_with(&buffer, "%begin") {
        // Beginning of output from a command we executed
        state.current_command = Some(command_queue.recv_blocking().unwrap());
    } else if buffer_starts_with(&buffer, "%end") {
        // End of output from a command we executed
        if let Some(current_command) = &state.current_command {
            match current_command {
                TmuxCommand::InitialOutput(pane_id) => {
                    let pane_id = *pane_id;
                    receive_event(
                        &event_channel,
                        TmuxEvent::ScrollOutput(pane_id, state.empty_line_count),
                    )?;
                    receive_event(&event_channel, TmuxEvent::InitialOutputFinished(pane_id))?;
                }
                TmuxCommand::ChangeSize(_, _) => {
                    receive_event(&event_channel, TmuxEvent::SizeChanged)?;
                }
                TmuxCommand::InitialLayout => {
                    receive_event(&event_channel, TmuxEvent::InitialLayoutFinished)?;
                }
                _ => {}
            }
        }

        state.current_command = None;
        state.is_error = false;
        state.result_line = 0;
        state.empty_line_count = 0;
    } else if buffer_starts_with(&buffer, "%error") {
        // TODO: We still don't actually print the error
        eprintln!("Error on command {:?}", state.current_command);

        // Command we executed produced an error
        state.current_command = None;
        state.is_error = false;
        state.result_line = 0;
        state.empty_line_count = 0;
    } else if buffer_starts_with(&buffer, "%window-pane-changed") {
        // %window-pane-changed @0 %10
        let (tab_id, chars_read) = read_first_u32(&buffer[22..]);
        let buffer = &buffer[22 + chars_read + 1..];
        let (pane_id, _) = read_first_u32(buffer);
        debug!(
            "Tmux event: Window {} focus changed to pane {}",
            tab_id, pane_id
        );
        receive_event(&event_channel, TmuxEvent::PaneFocusChanged(tab_id, pane_id))?;
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
        receive_event(&event_channel, TmuxEvent::TabFocusChanged(tab_id))?;
    } else if buffer_starts_with(&buffer, "%unlinked-window-close") {
        // %unlinked-window-close @6
        let (tab_id, _) = read_first_u32(&buffer[24..]);
        debug!("Tmux event: Tab {} closed", tab_id);
        receive_event(&event_channel, TmuxEvent::TabClosed(tab_id))?;
    } else if buffer_starts_with(&buffer, "%layout-change") {
        // Layout has changed
        let layout_sync = parse_tmux_layout(&buffer[15..]);
        receive_event(&event_channel, TmuxEvent::LayoutChanged(layout_sync))?;
    } else if buffer_starts_with(&buffer, "%session-changed") {
        // Session has changed
        let (id, bytes_read) = read_first_u32(&buffer[18..]);
        let name = parse_utf8(&buffer[18 + bytes_read..])?.to_string();
        debug!("Tmux event: Session changed ({}): {}", id, name);

        receive_event(&event_channel, TmuxEvent::SessionChanged(id, name))?;
    } else if buffer_starts_with(&buffer, "%window-renamed") {
        // Session has changed
        let (id, bytes_read) = read_first_u32(&buffer[17..]);
        let name = parse_utf8(&buffer[17 + bytes_read..])?.to_string();
        debug!("Tmux event: Tab renamed ({}): {}", id, name);

        receive_event(&event_channel, TmuxEvent::TabRenamed(id, name))?;
    } else if buffer_starts_with(&buffer, "%exit") {
        // Tmux client has exited
        let reason = parse_utf8(&buffer[5..])?;
        debug!("Tmux event: Exit received, reason: {}", reason);
        receive_event(&event_channel, TmuxEvent::Exit)?;
        // Stop receiving events
        return Err(TmuxError::ExitEventReceived);
    } else if buffer_starts_with(&buffer, "%client-session-changed") {
    } else {
        // Unsupported notification
        let notification = parse_utf8(&buffer)?;
        debug!("Tmux event: Unknown notification: {}", notification)
    }

    Ok(buffer.len())
}

#[inline]
fn tmux_command_result(
    command: &TmuxCommand,
    buffer: &[u8],
    result_line: usize,
    empty_lines: usize,
    event_channel: &Sender<TmuxEvent>,
    ssh_target: &Option<String>,
) -> Result<(), TmuxError> {
    match command {
        TmuxCommand::TabNew => {
            let layout_sync = parse_tmux_layout(buffer);
            receive_event(&event_channel, TmuxEvent::TabNew(layout_sync))?;
        }
        TmuxCommand::InitialLayout => {
            let layout_sync = parse_tmux_layout(buffer);
            receive_event(&event_channel, TmuxEvent::InitialLayout(layout_sync))?;
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

            receive_event(&event_channel, TmuxEvent::Output(*pane_id, output, true))?;
        }
        TmuxCommand::PaneCurrentPath(term_id) => {
            let (pane_id, bytes_read) = read_first_u32(&buffer[7..]);
            // Currently Tmux sends paths of all Terminals in the given Tab, so we need
            // to filter manually
            if pane_id == *term_id {
                let path = parse_utf8(&buffer[7 + bytes_read..])?;
                open_editor(path, ssh_target);
            }
        }
        _ => {}
    }

    Ok(())
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
