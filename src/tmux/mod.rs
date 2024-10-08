use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::str::{from_utf8, from_utf8_unchecked};

use async_channel::{Receiver, Sender};
use gtk4::gio::spawn_blocking;
use layout::{parse_tmux_layout, read_first_u32};
use vte4::GtkWindowExt;

use crate::error::IvyError;
use crate::window::IvyWindow;

mod layout;

// TODO: Implement command queue using channels
pub struct Tmux {
    stdin_stream: ChildStdin,
}

impl Tmux {
    pub fn send_command(&self, command: TmuxCommand) {
        let mut stdin_stream = &self.stdin_stream;
        match command {
            TmuxCommand::InitialLayout => {
                stdin_stream
                    .write_all(b"list-windows -F \"#{window_layout}\"\n")
                    .unwrap();
            }
        }
    }
}

enum TmuxEvent {
    Attached,
    OutputLine(String),
    Exit,
}

#[derive(Debug)]
pub enum TmuxCommand {
    InitialLayout,
}

pub fn attach_tmux(session_name: &str, window: &IvyWindow) -> Result<Tmux, IvyError> {
    // Create async channels
    let (tmux_event_sender, tmux_event_receiver): (Sender<TmuxEvent>, Receiver<TmuxEvent>) =
        async_channel::unbounded();

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
        tmux_read_stdout(stdout_stream, tmux_event_sender);
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
    let tmux = Tmux { stdin_stream };

    Ok(tmux)
}

#[inline]
fn tmux_event_future(event: TmuxEvent, window: &IvyWindow) {
    match event {
        TmuxEvent::Attached => {}
        TmuxEvent::OutputLine(line) => {}
        TmuxEvent::Exit => {
            println!("Received EXIT event, closing window!");
            window.close();
        }
    }
}

#[inline]
fn tmux_read_stdout(stdout_stream: ChildStdout, event_channel: Sender<TmuxEvent>) {
    let mut buffer = Vec::with_capacity(65534);
    let mut command_output = String::new();
    let mut reader = BufReader::new(stdout_stream);

    let mut current_command = TmuxCommand::InitialLayout;

    // TODO: Handle output larger than 65534 bytes
    while let Ok(bytes_read) = reader.read_until(10, &mut buffer) {
        if bytes_read == 0 {
            continue;
        }

        // All output from Tmux is acutally ASCII, except %output which we handle separately
        let line = unsafe { from_utf8_unchecked(&buffer) };

        if buffer[0] == b'%' {
            // This is a notification
            if line.starts_with("%output") {
                // We were given output, we can assume that up until pane_id, output is ASCII
                let (pane_id, chars_read) = read_first_u32(&buffer[9..]);
                let output = from_utf8(&buffer[chars_read..]).unwrap();
                println!("Output on Pane {}: {}", pane_id, output);
            } else if line.starts_with("%begin") {
            } else if line.starts_with("%layout-change") {
                // println!("Someone else is messing with our ")
            } else if line.starts_with("%end") {
                println!("Given command: ({:?}) ====\n{}", current_command, command_output);
            } else if line.starts_with("%error") {
                println!("Error: ({:?}) {}", current_command, command_output);
            } else if line.starts_with("%session-changed") {
                println!("Session changed: {}", &line[17..]);
            } else if line.starts_with("%exit") {
                println!("Exit: {}", &line[6..]);
                event_channel
                    .send_blocking(TmuxEvent::Exit)
                    .expect("Event channel closed!");
            } else {
                print!("Unknown notification: {}", line)
            }
        } else {
            // This is output from a command we ran
            command_output.push_str(line);
        }

        buffer.clear();
    }
    buffer.clear();
}
