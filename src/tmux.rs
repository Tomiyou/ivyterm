use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{ChildStdout, Command, Stdio};
use std::str::{from_utf8, from_utf8_unchecked};
use std::sync::mpsc::{self, Receiver, Sender};

use gtk4::gio::spawn_blocking;

use crate::error::IvyError;

enum TmuxEvent {
    Attached,
    OutputLine(String),
}

pub fn attach_tmux(session_name: &str) -> Result<(), IvyError> {
    println!("Attaching to tmux session {}", session_name);

    let process = Command::new("tmux")
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

    let stdout_pipe = process.stdout;
    let stdin_pipe = process.stdin;

    // Kako nej 1 thread hkrati posilja, hkrati sprejema?
    // Mogoce lahko matcha 2 kanala hkrati
    let (tmux_event_sender, tmux_event_receiver): (Sender<TmuxEvent>, Receiver<TmuxEvent>) =
        mpsc::channel();

    println!("Successfully spawned tmux");

    spawn_blocking(move || {
        // Read from tmux stdout and send events to channel
        if let Some(stdout_stream) = stdout_pipe {
            read_tmux_output(stdout_stream, tmux_event_sender);
        }
    });

    let mut tmux_input = BufWriter::new(stdin_pipe.unwrap());
    let status = tmux_input.write_all(b"send 'ls' Enter");
    println!("Hello darkness my old friend");

    Ok(())
}

#[derive(Debug)]
enum CurrentCommand {
    None,
    ReadingBlock,
    Success,
    Error,
}

fn read_tmux_output(stdout_stream: ChildStdout, tx_channel: Sender<TmuxEvent>) {
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
                let mut space_idx = 9;
                let mut pane_id = 0;
                loop {
                    // Find space first space character
                    if buffer[space_idx] == 32 {
                        break;
                    }
                    pane_id *= 10;
                    pane_id += buffer[space_idx] - 48;
                    space_idx += 1;
                }
                let output = from_utf8(&buffer[space_idx + 1..]).unwrap();
                println!("Output on Pane {}: {}", pane_id, output);
            } else if line.starts_with("%begin") {
            } else if line.starts_with("%end") {
                println!("Given command: ({:?}) {}", state, command_output);
                state = CurrentCommand::None;
            } else if line.starts_with("%error") {
                println!("Error: ({:?}) {}", state, command_output);
                state = CurrentCommand::None;
            } else if line.starts_with("%session-changed") {
                println!("Session changed: {}", &line[17..]);
            } else if line.starts_with("%exit") {
                println!("Exit: {}", &line[6..]);
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
