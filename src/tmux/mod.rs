use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{ChildStdout, Command, Stdio};
use std::rc::Rc;
use std::str::{from_utf8, from_utf8_unchecked};
use std::thread;
use std::time::Duration;

use async_channel::{Receiver, Sender};
use glib::clone;
use gtk4::gio::spawn_blocking;
use layout::{parse_tmux_layout, read_first_u32};
use vte4::GtkWindowExt;

use crate::error::IvyError;
use crate::global_state::WindowState;

mod layout;

enum TmuxEvent {
    Attached,
    OutputLine(String),
    Exit,
}

enum IvyEvent {}

pub fn attach_tmux(
    session_name: &str,
    window_state: &Rc<WindowState>,
) -> Result<(), IvyError> {
    // Create async channels
    let (event_sender, event_receiver): (Sender<TmuxEvent>, Receiver<TmuxEvent>) =
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
        tmux_read_stdout(stdout_stream, event_sender);
    });
    // Receive events from the channel on main thread
    let window_state = window_state.clone();
    glib::spawn_future_local(async move {
        while let Ok(event) = event_receiver.recv().await {
            tmux_event_future(event, &window_state);
        }
    });

    let wawa = "191x47,0,0[191x23,0,0,0,191x23,0,24{95x23,0,24,1,95x23,96,24,2}]";
    parse_tmux_layout(wawa.as_bytes());

    // Handle Tmux STDIN
    // The main loop executes the asynchronous block

    if let Some(stdin) = process.stdin.as_mut() {
        println!("Writing to STDIN!");
        stdin.write_all(b"list-panes\n").unwrap();
        let five_seconds = Duration::from_secs(5);
        thread::sleep(five_seconds);
    }

    Ok(())
}

#[inline]
fn tmux_event_future(event: TmuxEvent, window_state: &Rc<WindowState>) {
    match event {
        TmuxEvent::Attached => {}
        TmuxEvent::OutputLine(line) => {}
        TmuxEvent::Exit => {
            println!("Received EXIT event, closing window!");
            window_state.window.close();
        }
    }
}

#[derive(Debug)]
enum CurrentCommand {
    None,
    ReadingBlock,
    Success,
    Error,
}

#[inline]
fn tmux_read_stdout(stdout_stream: ChildStdout, event_channel: Sender<TmuxEvent>) {
    let mut buffer = Vec::with_capacity(65534);
    let mut command_output = String::new();
    let mut reader = BufReader::new(stdout_stream);

    let mut state = CurrentCommand::None;

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
                println!("Given command: ({:?}) ====\n{}", state, command_output);
                state = CurrentCommand::None;
            } else if line.starts_with("%error") {
                println!("Error: ({:?}) {}", state, command_output);
                state = CurrentCommand::None;
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
