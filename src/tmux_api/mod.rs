use std::process::{ChildStdin, Command, Stdio};

use async_channel::{Receiver, Sender};
use gtk4::gio::spawn_blocking;
use gtk4::Orientation;
use receive::tmux_read_stdout;

use crate::helpers::IvyError;
use crate::keyboard::Direction;
use crate::tmux_widgets::IvyTmuxWindow;

mod parse_layout;
mod receive;
mod send;

pub struct TmuxAPI {
    stdin_stream: ChildStdin,
    command_queue: Sender<TmuxCommand>,
    window_size: (i32, i32),
    resize_future: bool,
    pub initial_output: TmuxTristate,
}

#[derive(PartialEq)]
pub enum TmuxTristate {
    Uninitialized,
    WaitingResponse,
    Done,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum TmuxCommand {
    Init,
    InitialLayout,
    Keypress,
    TabNew,
    TabClose,
    PaneSplit(bool),
    PaneClose(u32),
    PaneSelect(Direction),
    ChangeSize(i32, i32),
    InitialOutput(u32),
}

pub enum TmuxEvent {
    ScrollOutput(u32, usize),
    InitialLayout(u32, Vec<TmuxPane>),
    InitialOutputFinished(),
    LayoutChanged(u32, Vec<TmuxPane>),
    Output(u32, Vec<u8>),
    SizeChanged(),
    PaneSplit(u32, Vec<TmuxPane>),
    FocusChanged(u32),
    Exit,
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

impl TmuxAPI {
    pub fn new(session_name: &str, window: &IvyTmuxWindow) -> Result<TmuxAPI, IvyError> {
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
        let tmux = TmuxAPI {
            stdin_stream: stdin_stream,
            command_queue: cmd_queue_sender,
            window_size: (0, 0),
            resize_future: false,
            initial_output: TmuxTristate::Uninitialized,
        };

        Ok(tmux)
    }
}
