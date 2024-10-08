use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::str::{from_utf8, from_utf8_unchecked};

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
                    .write_all(b"list-windows -F \"#{window_layout}\"\n")
                    .unwrap();
            }
            TmuxCommand::ChangeSize(cols, rows) => {
                debug!("Resizing Tmux client to {}x{}", cols, rows);
                let cmd = format!("refresh-client -C {},{}\n", cols, rows);
                command_queue.send_blocking(command).unwrap();
                stdin_stream.write_all(cmd.as_bytes()).unwrap();
            }
            TmuxCommand::InitialOutput(pane_id) => {
                debug!("Getting initial output of pane {}", pane_id);
                let cmd = format!("capture-pane -J -p -t {} -eC -S - -E -\n", pane_id);
                command_queue.send_blocking(command).unwrap();
                stdin_stream.write_all(cmd.as_bytes()).unwrap();
            }
            _ => {}
        }
    }

    pub fn send_keypress(&self, pane_id: u32, c: char, prefix: String, control: Option<&str>) {
        let command_queue = &self.command_queue;
        let mut stdin_stream = &self.stdin_stream;

        let cmd = if let Some(control) = control {
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
    ChangeSize(i64, i64),
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

// #[inline]
fn parse_escaped_output(input: &[u8], prepend_newline: bool) -> Vec<u8> {
    let input_len = input.len();
    let mut output = Vec::with_capacity(input_len);

    if prepend_newline {
        output.push(b'\r');
        output.push(b'\n');
    }

    let mut i = 0;
    while i < input_len {
        let char = input[i];
        if char == b'\\' {
            // Maybe an escape sequence?
            if i + 3 >= input_len {
                panic!("Found escape character but string too short");
            }

            // This is an escape sequence
            // TODO: This crashes when output is:
            // tomi:~/plume/opensync/sdk/device-sdk-qca (master) $ danes je pa tako M-\M-\
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
    let mut output_line = 0;
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

        // TODO: Probably not OK for initial output?
        if buffer.is_empty() {
            continue;
        }

        if buffer[0] != b'%' {
            // This is output from a command we ran
            if is_error {
                let error = from_utf8(&buffer).unwrap();
                println!("Error: ({:?}) {}", current_command, error);
            } else {
                if let Some(command) = &current_command {
                    tmux_command_response(command, &buffer, output_line, &event_channel);
                }
            }

            output_line += 1;
        } else {
            // This is a notification
            if buffer_starts_with(&buffer, "%output") {
                // We were given output, we can assume that up until pane_id, output is ASCII
                let (pane_id, chars_read) = read_first_u32(&buffer[9..]);
                let output = parse_escaped_output(&buffer[9 + chars_read..], false);

                event_channel
                    .send_blocking(TmuxEvent::Output(pane_id, output))
                    .expect("Event channel closed!");
                buffer.clear();
            } else if buffer_starts_with(&buffer, "%begin") {
                // Beginning of output from a command we executed
                current_command = Some(command_queue.recv_blocking().unwrap());
            } else if buffer_starts_with(&buffer, "%end") {
                // End of output from a command we executed
                current_command = None;
                output_line = 0;
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

fn tmux_command_response(
    command: &TmuxCommand,
    buffer: &[u8],
    line: u32,
    event_channel: &Sender<TmuxEvent>,
) {
    match command {
        TmuxCommand::InitialLayout => {
            // let bytes = output.as_bytes();
            // parse_tmux_layout(bytes);
            let layout = String::from_utf8(buffer.to_vec()).unwrap();
            event_channel
                .send_blocking(TmuxEvent::LayoutChanged(layout))
                .unwrap();
        }
        TmuxCommand::InitialOutput(pane_id) => {
            // Skip the first line of output
            // TODO: What do we do with this?
            // if line == 0 {
            //     return;
            // }

            let output = parse_escaped_output(buffer, line >= 2);
            event_channel
                .send_blocking(TmuxEvent::Output(*pane_id, output))
                .expect("Event channel closed!");
        }
        _ => {}
    }
}
