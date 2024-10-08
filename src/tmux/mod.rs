use std::ascii::escape_default;
use std::borrow::BorrowMut;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::str::from_utf8;

use async_channel::{Receiver, Sender};
use gtk4::gio::spawn_blocking;
use layout::{parse_tmux_layout, read_first_u32};
use log::debug;
use vte4::GtkWindowExt;

use crate::error::IvyError;
use crate::window::IvyWindow;

mod layout;

// TODO: Implement command queue using channels
pub struct Tmux {
    stdin_stream: ChildStdin,
    command_queue: Sender<TmuxCommand>,
    window_size: (u32, u32),
}

impl Tmux {
    pub fn send_command(&self, command: TmuxCommand) {
        let command_queue = &self.command_queue;
        let mut stdin_stream = &self.stdin_stream;

        match command {
            TmuxCommand::InitialLayout => {
                debug!("Getting initial layout");
                command_queue.send_blocking(command).unwrap();
                stdin_stream
                    .write_all(b"list-windows -F \"#{window_layout},#{history_size}\"\n")
                    .unwrap();
            }
            TmuxCommand::InitialOutput(pane_id) => {
                debug!("Getting initial output of pane {}", pane_id);
                let cmd = format!("capture-pane -J -p -t {} -eC -S - -E -\n", pane_id);
                command_queue.send_blocking(command).unwrap();
                stdin_stream.write_all(cmd.as_bytes()).unwrap();
            }
            _ => {
                panic!("Don't know how to handle command {:?}", command);
            }
        }
    }

    pub fn change_size(&mut self, cols: u32, rows: u32) {
        self.window_size = (cols, rows);

        let command = TmuxCommand::ChangeSize(cols, rows);

        debug!("Resizing Tmux client to {}x{}", cols, rows);
        let cmd = format!("refresh-client -C {},{}\n", cols, rows);
        self.command_queue.send_blocking(command).unwrap();
        self.stdin_stream.write_all(cmd.as_bytes()).unwrap();
    }

    pub fn send_keypress(&self, pane_id: u32, c: char, prefix: String, movement: Option<&str>) {
        let command_queue = &self.command_queue;
        let mut stdin_stream = &self.stdin_stream;

        let cmd = if let Some(control) = movement {
            // Navigation keys (left, right, page up, ...)
            format!("send-keys -t {} {}{}\n", pane_id, prefix, control)
        } else if c.is_ascii_control() {
            // A control character was just pressed
            let ascii = c as u8;
            format!("send-keys -t {} -- {}\\{:03o}\n", pane_id, prefix, ascii)
        } else {
            // We send single-quoted keys, but what if we want to send a single quote?
            let quote = if c == '\'' { '"' } else { '\'' };

            // If Ctrl/Shift/Alt was pressed, prefix will not be empty and we need to
            // remove Tmux's -l flag
            let flags = if prefix.is_empty() { "-l" } else { "" };

            format!(
                "send-keys -t {} {} -- {}{}{}{}\n",
                pane_id, flags, quote, prefix, c, quote
            )
        };

        debug!("send_keypress: {}", &cmd[..cmd.len() - 1]);
        command_queue.send_blocking(TmuxCommand::Keypress).unwrap();
        stdin_stream.write_all(cmd.as_bytes()).unwrap();
    }
}

enum TmuxEvent {
    Attached,
    LayoutChanged(String),
    Output(u32, Vec<u8>),
    Exit,
}

#[derive(Debug)]
pub enum TmuxCommand {
    Init,
    InitialLayout,
    Keypress,
    ChangeSize(u32, u32),
    InitialOutput(u32),
}

pub fn attach_tmux(session_name: &str, window: &IvyWindow) -> Result<Tmux, IvyError> {
    // Create async channels
    let (tmux_event_sender, tmux_event_receiver): (Sender<TmuxEvent>, Receiver<TmuxEvent>) =
        async_channel::unbounded();

    // Command queue
    let (cmd_queue_sender, cmd_queue_receiver): (Sender<TmuxCommand>, Receiver<TmuxCommand>) =
        async_channel::unbounded();
    // Parse attach output
    cmd_queue_sender.send_blocking(TmuxCommand::Init).unwrap();

    // Spawn TMUX subprocess
    println!("Attaching to tmux session {}", session_name);
    let mut process = Command::new("tmux")
        .arg("-2")
        .arg("-C")
        .arg("attach-session")
        .arg("-t")
        .arg(session_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Read from Tmux STDOUT and send events to the channel on a separate thread
    let stdout_stream = process.stdout.take().expect("Failed to open stdout");
    spawn_blocking(move || {
        tmux_read_stdout(stdout_stream, tmux_event_sender, cmd_queue_receiver);
    });
    // Receive events from the channel on main thread
    let window = window.clone();
    glib::spawn_future_local(async move {
        while let Ok(event) = tmux_event_receiver.recv().await {
            tmux_event_future(event, &window);
        }
    });

    // Handle Tmux STDIN
    let stdin_stream = process.stdin.take().expect("Failed to open stdin");
    let tmux = Tmux {
        stdin_stream: stdin_stream,
        command_queue: cmd_queue_sender,
        window_size: (0, 0),
    };

    Ok(tmux)
}

#[inline]
fn tmux_event_future(event: TmuxEvent, window: &IvyWindow) {
    // This future runs on main thread of GTK application
    // It receives Tmux events from separate thread and runs GTK functions
    match event {
        TmuxEvent::Attached => {}
        TmuxEvent::LayoutChanged(layout) => {
            window.tmux_resize_window();
            window.tmux_inital_output();
        }
        TmuxEvent::Output(pane_id, output) => {
            window.output_on_pane(pane_id, output);
        }
        TmuxEvent::Exit => {
            println!("Received EXIT event, closing window!");
            window.close();
        }
    }
}

/// Parses Tmux output and replaces octal escapes sequences with correct binary
/// characters
#[inline]
fn parse_tmux_output<'a>(
    input: impl Iterator<Item = &'a u8>,
    input_len: usize,
) -> (Vec<u8>, usize) {
    let mut input = input;
    let mut output = Vec::with_capacity(input_len);

    let mut bytes_read = 0;
    while let Some(char) = input.next() {
        let char = *char;
        if char == b'\\' {
            // First \ is an escape, meaning there is 100% another char after it
            let char = *input.next().unwrap();
            if char == b'\\' {
                // First \ is followed by another \
                output.push(b'\\');
                bytes_read += 2;
                continue;
            }

            // This is an escape sequence
            // TODO: This crashes when output is:
            // tomi:~/plume/opensync/sdk/device-sdk-qca (master) $ danes je pa tako M-\M-\
            let mut ascii = char - 48;
            for _ in 0..2 {
                let num = *input.next().unwrap() - 48;
                ascii *= 8;
                ascii += num;
            }
            output.push(ascii);
            bytes_read += 4;
        } else {
            output.push(char);
            bytes_read += 1;
        }

        if char == b'\n' {
            // We stop at \n
            break;
        }
    }

    (output, bytes_read)
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

struct InitialOutputState {
    buffer: VecDeque<u8>,
    lines: u32,
    max_lines: u32,
    empty_lines_end: u32,
}

#[inline]
fn tmux_read_stdout(
    stdout_stream: ChildStdout,
    event_channel: Sender<TmuxEvent>,
    command_queue: Receiver<TmuxCommand>,
) {
    let mut buffer = Vec::with_capacity(65534);
    let mut initial_output = InitialOutputState {
        buffer: VecDeque::new(),
        lines: 0,
        max_lines: 0,
        empty_lines_end: 0,
    };
    let mut reader = BufReader::new(stdout_stream);

    // Variable containing Tmux state
    let mut current_command = None;
    let mut is_error = false;

    // TODO: Handle output larger than 65534 bytes
    while let Ok(bytes_read) = reader.read_until(10, &mut buffer) {
        // All output from Tmux is ASCII, except %output which we handle separately
        if bytes_read == 0 {
            continue;
        }

        // Since we read until (and including) '\n', it will always be at the
        // end of the buffer. We should strip it here.
        buffer.pop();
        debug!("Tmux output: {}", from_utf8(&buffer).unwrap());
        if false {
            let mut escaped = String::with_capacity(buffer.len() * 2);
            for c in &buffer {
                if c.is_ascii_control() {
                    let x = escape_default(*c);
                    for c in x {
                        escaped.push(c as char);
                    }
                } else {
                    escaped.push(*c as char);
                }
            }
            println!("Tmux output: |{}|", escaped);
        }

        // Don't print empty lines at the end of output
        if buffer.is_empty() || buffer[0] != b'%' {
            // This is output from a command we ran
            if is_error {
                let error = from_utf8(&buffer).unwrap();
                println!("Error: ({:?}) {}", current_command, error);
            } else if let Some(command) = &current_command {
                tmux_command_response(command, &buffer, &event_channel, &mut initial_output, false);
            }
        } else {
            // This is a notification
            if buffer_starts_with(&buffer, "%output") {
                // We were given output, we can assume that up until pane_id, output is ASCII
                let (pane_id, chars_read) = read_first_u32(&buffer[9..]);
                let remaining = &buffer[9 + chars_read..];
                let (output, _) = parse_tmux_output(remaining.iter(), remaining.len());

                event_channel
                    .send_blocking(TmuxEvent::Output(pane_id, output))
                    .expect("Event channel closed!");
            } else if buffer_starts_with(&buffer, "%begin") {
                // Beginning of output from a command we executed
                current_command = Some(command_queue.recv_blocking().unwrap());
            } else if buffer_starts_with(&buffer, "%end") {
                // End of output from a command we executed
                if let Some(current_command) = current_command {
                    tmux_command_response(
                        &current_command,
                        &buffer,
                        &event_channel,
                        &mut initial_output,
                        true,
                    );
                }

                current_command = None;
                is_error = false;
            } else if buffer_starts_with(&buffer, "%layout-change") {
                // Layout has changed
            } else if buffer_starts_with(&buffer, "%error") {
                // Command we executed produced an error
                current_command = Some(command_queue.recv_blocking().unwrap());
                is_error = true;
            } else if buffer_starts_with(&buffer, "%session-changed") {
                // Session has changed
                let session = from_utf8(&buffer[17..]).unwrap();
                println!("Session changed: {}", session);
            } else if buffer_starts_with(&buffer, "%exit") {
                // Tmux client has exited
                let reason = from_utf8(&buffer[5..]).unwrap();
                println!("Exit received, reason:{}", reason);
                event_channel
                    .send_blocking(TmuxEvent::Exit)
                    .expect("Event channel closed!");
            } else {
                // Unsupported notification
                let notification = from_utf8(&buffer).unwrap();
                print!("Unknown notification: {}", notification)
            }
        }

        buffer.clear();
    }
    buffer.clear();
}

#[inline]
fn tmux_command_response(
    command: &TmuxCommand,
    buffer: &[u8],
    event_channel: &Sender<TmuxEvent>,
    initial_output: &mut InitialOutputState,
    end: bool,
) {
    match (command, end) {
        (TmuxCommand::InitialLayout, false) => {
            // let bytes = output.as_bytes();
            // parse_tmux_layout(bytes);
            let layout = String::from_utf8(buffer.to_vec()).unwrap();
            event_channel
                .send_blocking(TmuxEvent::LayoutChanged(layout))
                .unwrap();
        }
        (TmuxCommand::InitialOutput(pane_id), end) => {
            let ring_buffer = initial_output.buffer.borrow_mut();

            if end {
                // Clear screen (but keep history)
                ring_buffer.pop_back();
                ring_buffer.pop_back();
                for b in b"J2[\x1bH[\x1b" {
                    // for b in b"\x1b[H\x1b[2J" {
                    ring_buffer.push_front(*b);
                }

                let mut blabla = Vec::new();
                for b in ring_buffer.iter() {
                    if b.is_ascii_control() {
                        for x in escape_default(*b) {
                            blabla.push(x);
                        }
                    } else {
                        blabla.push(*b);
                    }
                }
                println!("This was STILL in ringbuffer: \n{}", from_utf8(&blabla).unwrap());

                // Flush anything remaining in the ringbuffer, but ignore empty lines at the end
                for _ in 0..initial_output.empty_lines_end + 1 {
                    ring_buffer.pop_back(); // Pop \n
                    ring_buffer.pop_back(); // Pop \r
                }

                let len = ring_buffer.len();
                let (line, _) = parse_tmux_output(ring_buffer.iter(), len);
                event_channel
                    .send_blocking(TmuxEvent::Output(*pane_id, line))
                    .expect("Event channel closed!");

                initial_output.buffer.clear();
                initial_output.lines = 0;
                initial_output.empty_lines_end = 0;
                return;
            }

            // If ringbuffer is full, we need to remove a line from ringbuffer and print it on the screen
            if initial_output.lines >= initial_output.max_lines {
                let mut blabla = Vec::new();
                for b in ring_buffer.iter() {
                    if b.is_ascii_control() {
                        for x in escape_default(*b) {
                            blabla.push(x);
                        }
                    } else {
                        blabla.push(*b);
                    }
                }
                // println!("This was in ringbuffer: {}", from_utf8(&blabla).unwrap());

                // TODO: Get size of the line using O(n)
                let len = ring_buffer.len();
                let (line, bytes_read) = parse_tmux_output(ring_buffer.iter(), len);
                event_channel
                    .send_blocking(TmuxEvent::Output(*pane_id, line))
                    .expect("Event channel closed!");

                ring_buffer.drain(..bytes_read);

                // TODO: Remove line from ringbuffer
                initial_output.lines -= 1;
            }

            // Keep track of empty lines at the end of pane output
            if buffer.is_empty() {
                initial_output.empty_lines_end += 1;
            } else {
                initial_output.empty_lines_end = 0;
            }

            println!("Given output {}", from_utf8(buffer).unwrap());
            // Append the buffer to ringbuffer
            for b in buffer {
                ring_buffer.push_back(*b);
            }
            ring_buffer.push_back(b'\r');
            ring_buffer.push_back(b'\n');
            initial_output.lines += 1;
        }
        (TmuxCommand::ChangeSize(cols, rows), true) => {
            initial_output.max_lines = *rows;

            // If buffer is already large enough, leave it be
            let desired_capacity = ((rows + 1) * cols * 4) as usize;
            if initial_output.buffer.capacity() < desired_capacity {
                initial_output.buffer = VecDeque::with_capacity(desired_capacity)
            }
        }
        _ => {}
    }
}
