use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::str::from_utf8;

use async_channel::{Receiver, Sender};
use gtk4::gio::spawn_blocking;
use log::debug;

use crate::helpers::IvyError;
use crate::window::IvyWindow;

#[derive(PartialEq)]
pub enum TmuxTristate {
    Uninitialized,
    WaitingResponse,
    Done,
}

pub struct Tmux {
    stdin_stream: ChildStdin,
    command_queue: Sender<TmuxCommand>,
    window_size: (i32, i32),
    resize_future: bool,
    pub initial_output: TmuxTristate,
}

impl Tmux {
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
        let cmd = format!("capture-pane -J -p -t {} -eC -S - -E -\n", pane_id);
        command_queue.send_blocking(TmuxCommand::InitialOutput(pane_id)).unwrap();
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

        println!("send_keypress: {}", &cmd[..cmd.len() - 1]);
        command_queue.send_blocking(TmuxCommand::Keypress).unwrap();
        stdin_stream.write_all(cmd.as_bytes()).unwrap();
    }

    /// Updates resize_future to `new` value, while returning the old value
    pub fn update_resize_future(&mut self, new: bool) -> bool {
        let old = self.resize_future;
        self.resize_future = new;
        return old;
    }
}

#[derive(Debug)]
pub enum TmuxCommand {
    Init,
    InitialLayout,
    Keypress,
    ChangeSize(i32, i32),
    InitialOutput(u32),
}

pub enum TmuxEvent {
    ScrollOutput(u32, usize),
    InitialLayout(Vec<u8>),
    InitialOutputFinished(),
    LayoutChanged(Vec<u8>),
    Output(u32, Vec<u8>),
    SizeChanged(),
    Exit,
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
    let _window = window.clone();
    glib::spawn_future_local(async move {
        while let Ok(event) = tmux_event_receiver.recv().await {
            _window.tmux_event_callback(event)
        }
    });

    // Handle Tmux STDIN
    let stdin_stream = process.stdin.take().expect("Failed to open stdin");
    let tmux = Tmux {
        stdin_stream: stdin_stream,
        command_queue: cmd_queue_sender,
        window_size: (0, 0),
        resize_future: false,
        initial_output: TmuxTristate::Uninitialized,
    };

    Ok(tmux)
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
fn buffer_starts_with(buffer: &Vec<u8>, prefix: &str) -> bool {
    if prefix.len() > buffer.len() {
        return false;
    }

    let buffer = &buffer[..prefix.len()];
    let prefix = prefix.as_bytes();

    buffer == prefix
}

#[inline]
fn tmux_read_stdout(
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
                    .send_blocking(TmuxEvent::Output(pane_id, output))
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
                                .send_blocking(TmuxEvent::InitialOutputFinished())
                                .expect("Event channel closed!");
                        }
                        TmuxCommand::ChangeSize(_, _) => {
                            event_channel
                                .send_blocking(TmuxEvent::SizeChanged())
                                .expect("Event channel closed!");
                        }
                        _ => {}
                    }
                }

                current_command = None;
                is_error = false;
                result_line = 0;
                empty_line_count = 0;
            } else if buffer_starts_with(&buffer, "%layout-change") {
                // Layout has changed
                event_channel
                    .send_blocking(TmuxEvent::LayoutChanged(buffer.clone()))
                    .unwrap();
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
            } else if buffer_starts_with(&buffer, "%client-session-changed") {
                
            } else {
                // Unsupported notification
                let notification = from_utf8(&buffer).unwrap();
                println!("Unknown notification: {}", notification)
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
        TmuxCommand::InitialLayout => {
            event_channel
                .send_blocking(TmuxEvent::InitialLayout(buffer.clone()))
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
                .send_blocking(TmuxEvent::Output(*pane_id, output))
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
    while buffer[i] > 47 && buffer[i] < 58 {
        number *= 10;
        number += (buffer[i] - 48) as u32;
        i += 1;
    }
    (number, i + 1)
}
