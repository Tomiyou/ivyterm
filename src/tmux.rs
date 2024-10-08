use std::{
    io::{BufRead, BufReader},
    process::{ChildStdout, Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

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
        // .stdin(Stdio::piped())
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
            read_tmux_output(stdout_stream);
        }
    });

    // let kekw = stdin_pipe
    //     .unwrap()
    //     .write_fmt(format_args!("{:.*}", 2, 1.234567));
    // println!("Hello darkness my old friend");

    Ok(())
}

#[derive(Debug)]
enum CurrentCommand {
    None,
    ReadingBlock,
    Success,
    Error,
}

const TMUX_POLL_MILLIS: Duration = Duration::from_millis(10);

fn read_tmux_output(stdout_stream: ChildStdout) {
    let mut buffer = String::new();
    let mut command_output = String::new();
    let mut reader = BufReader::new(stdout_stream);

    let mut state = CurrentCommand::None;

    while let Ok(bytes_read) = reader.read_line(&mut buffer) {
        if bytes_read != 0 {
            if buffer.starts_with('%') {
                // This is a notification

                if buffer.starts_with("%output") {
                    // We were given output
                    let line = &buffer[9..];
                    let mut space_idx = 0;
                    for character in line.bytes() {
                        // Find space first space character
                        if character == 32 {
                            break;
                        }
                        space_idx += 1;
                    }
                    if space_idx == 0 {
                        panic!("space_idx cannot be 0! {}", line);
                    }
                    let pane_id: u16 = line[..space_idx].parse().unwrap();
                    println!("Output on Pane {}: {}", pane_id, &line[space_idx + 1..]);
                } else if buffer.starts_with("%begin") {
                } else if buffer.starts_with("%end") {
                    println!("Given command: ({:?}) {}", state, command_output);
                    state = CurrentCommand::None;
                } else if buffer.starts_with("%error") {
                    println!("Error: ({:?}) {}", state, command_output);
                    state = CurrentCommand::None;
                } else if buffer.starts_with("%session-changed") {
                    println!("Session changed: {}", &buffer[17..]);
                } else if buffer.starts_with("%exit") {
                    println!("Exit: {}", &buffer[6..]);
                } else {
                    print!("Unknown notification: {}", buffer)
                }
            } else {
                // This is output from a command we ran
                command_output.push_str(&buffer);
            }

            buffer.clear();
        }

        // TODO: Polling is probably a bad idea
        std::thread::sleep(TMUX_POLL_MILLIS);
    }
    buffer.clear();
}
