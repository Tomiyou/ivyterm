use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

use async_channel::{Receiver, Sender};
use enumflags2::{bitflags, BitFlags};
use gtk4::gio::spawn_blocking;
use gtk4::Orientation;
use mio::{Events, Poll};
use receive::tmux_parse_line;
use ssh2::Session;

use crate::helpers::IvyError;
use crate::keyboard::Direction;
use crate::ssh::SSH_TOKEN;
use crate::tmux_widgets::IvyTmuxWindow;

mod parse_layout;
mod receive;
mod send;

pub struct TmuxAPI {
    stdin_stream: Box<dyn Write>,
    command_queue: Sender<TmuxCommand>,
    window_size: (i32, i32),
    resize_future: bool,
}

impl Drop for TmuxAPI {
    fn drop(&mut self) {
        println!("Dropping TMUX");
    }
}

pub struct LayoutSync {
    pub tab_id: u32,
    pub layout: Vec<TmuxPane>,
    pub visible_layout: Vec<TmuxPane>,
    pub flags: BitFlags<LayoutFlags>,
    pub name: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum TmuxCommand {
    Init,
    InitialLayout,
    Keypress,
    TabNew,
    TabClose,
    TabSelect(u32),
    TabRename(u32),
    PaneSplit(bool),
    PaneClose(u32),
    PaneSelect(u32),
    PaneCurrentPath(u32),
    PaneMoveFocus(Direction),
    PaneZoom(u32),
    ChangeSize(i32, i32),
    InitialOutput(u32),
    ClipboardPaste,
}

pub enum TmuxEvent {
    ScrollOutput(u32, usize),
    InitialLayout(LayoutSync),
    InitialLayoutFinished,
    InitialOutputFinished(u32),
    LayoutChanged(LayoutSync),
    Output(u32, Vec<u8>, bool),
    SizeChanged,
    PaneFocusChanged(u32, u32),
    TabFocusChanged(u32),
    TabNew(LayoutSync),
    TabClosed(u32),
    TabRenamed(u32, String),
    SessionChanged(u32, String),
    Exit,
}

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LayoutFlags {
    HasFocus,
    IsZoomed,
}

#[derive(Debug, Clone, Copy)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug)]
pub enum TmuxPane {
    Terminal(u32, Rectangle),
    /// Has tuple (is_vertical, bounds)
    Container(Orientation, Rectangle),
    Return,
}

struct TmuxParserState {
    ssh_target: Option<String>,
    event_channel: Sender<TmuxEvent>,
    command_queue: Receiver<TmuxCommand>,
    current_command: Option<TmuxCommand>,
    is_error: bool,
    result_line: usize,
    empty_line_count: usize,
}

impl TmuxParserState {
    fn new(
        tmux_event_sender: Sender<TmuxEvent>,
        cmd_queue_receiver: Receiver<TmuxCommand>,
    ) -> Self {
        Self {
            command_queue: cmd_queue_receiver,
            event_channel: tmux_event_sender,
            current_command: None,
            is_error: false,
            ssh_target: None,
            result_line: 0,
            empty_line_count: 0,
        }
    }
}

impl TmuxAPI {
    pub fn new(
        session_name: &str,
        ssh_session: Option<(Session, Poll, Events)>,
        window: &IvyTmuxWindow,
    ) -> Result<TmuxAPI, IvyError> {
        // Create async channels
        let (tmux_event_sender, tmux_event_receiver): (Sender<TmuxEvent>, Receiver<TmuxEvent>) =
            async_channel::unbounded();

        // Command queue
        let (cmd_queue_sender, cmd_queue_receiver): (Sender<TmuxCommand>, Receiver<TmuxCommand>) =
            async_channel::unbounded();
        // Parse attach output
        cmd_queue_sender.send_blocking(TmuxCommand::Init).unwrap();

        // Spawn TMUX subprocess
        let spawn = if let Some(tuple) = ssh_session {
            new_with_ssh(session_name, tuple, tmux_event_sender, cmd_queue_receiver)
        } else {
            new_without_ssh(session_name, tmux_event_sender, cmd_queue_receiver)
        };
        let writer = match spawn {
            Err(_) => return Err(IvyError::Blabla),
            Ok(writer) => writer,
        };

        // Receive events from the channel on main thread
        glib::spawn_future_local(glib::clone!(
            #[weak]
            window,
            async move {
                while let Ok(event) = tmux_event_receiver.recv().await {
                    window.tmux_event_callback(event)
                }
            }
        ));

        // Handle Tmux STDIN
        let tmux = TmuxAPI {
            stdin_stream: writer,
            command_queue: cmd_queue_sender,
            window_size: (0, 0),
            resize_future: false,
        };

        Ok(tmux)
    }
}

fn new_with_ssh(
    session_name: &str,
    tuple: (Session, Poll, Events),
    tmux_event_sender: Sender<TmuxEvent>,
    cmd_queue_receiver: Receiver<TmuxCommand>,
) -> Result<Box<dyn Write>, ()> {
    let (session, mut poll, mut events) = tuple;

    let command = format!("tmux -2 -C new-session -A -s {}", session_name);
    let mut channel = session.channel_session().unwrap();
    channel
        .exec(&command)
        .unwrap();
    session.set_blocking(false);

    let ssh_stdin = channel.stream(0);
    let mut ssh_stdout = channel.stream(0);
    let mut ssh_stderr = channel.stderr();

    spawn_blocking(move || {
        let mut buffer = vec![0; 65534];
        let mut state = TmuxParserState::new(tmux_event_sender, cmd_queue_receiver);

        loop {
            poll.poll(&mut events, None).unwrap();
            for event in events.iter() {
                match event.token() {
                    SSH_TOKEN => {
                        if event.is_readable() {
                            match ssh_stdout.read(&mut buffer) {
                                Ok(bytes_read) => {
                                    let mut slice = &buffer[..bytes_read];

                                    let mut i = 0;
                                    while i < slice.len() {
                                        if slice[i] == b'\n' {
                                            tmux_parse_line(&mut state, &slice[..i]);
                                            slice = &slice[i + 1..];
                                            i = 0;
                                            continue;
                                        }
                                        i += 1;
                                    }
                                    if !slice.is_empty() {
                                        tmux_parse_line(&mut state, slice);
                                    }
                                }
                                Err(e) => {
                                    if e.kind() != std::io::ErrorKind::WouldBlock {
                                        println!("Error reading Tmux stdout: {}", e);
                                        return;
                                    }
                                }
                            }

                            let stderr = io::stderr();
                            let mut stderr = stderr.lock();
                            let mut buf = vec![0; 4096];
                            match ssh_stderr.read(&mut buf) {
                                Ok(_) => {
                                    let s = String::from_utf8(buf).unwrap();
                                    stderr.write(s.as_bytes()).unwrap();
                                }
                                Err(e) => {
                                    if e.kind() != std::io::ErrorKind::WouldBlock {
                                        println!("Stderr: {}", e);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    _ => unreachable!(),
                }

                if event.is_error() {
                    println!("Event is error, quitting!!!");
                    return;
                }
                if event.is_read_closed() || event.is_write_closed() {
                    println!("Read or write closed, quitting!!!");
                    return;
                }
            }
        }
    });

    return Ok(Box::new(ssh_stdin));
}

fn new_without_ssh(
    session_name: &str,
    tmux_event_sender: Sender<TmuxEvent>,
    cmd_queue_receiver: Receiver<TmuxCommand>,
) -> Result<Box<dyn Write>, ()> {
    println!("Attaching to Tmux session {}", session_name);
    let mut process = Command::new("tmux")
        .arg("-2")
        .arg("-C")
        .arg("new-session")
        .arg("-A")
        .arg("-s")
        .arg(session_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Read from Tmux STDOUT and send events to the channel on a separate thread
    let stdout_stream = process.stdout.take().expect("Failed to open stdout");
    spawn_blocking(move || {
        let mut buffer = Vec::with_capacity(65534);
        let mut reader = BufReader::new(stdout_stream);
        let mut state = TmuxParserState::new(tmux_event_sender, cmd_queue_receiver);

        while let Ok(bytes_read) = reader.read_until(10, &mut buffer) {
            tmux_parse_line(&mut state, &buffer[..bytes_read - 1]);
            buffer.clear();
        }
    });

    let stdin_stream = process.stdin.take().expect("Failed to open stdin");
    return Ok(Box::new(stdin_stream));
}
