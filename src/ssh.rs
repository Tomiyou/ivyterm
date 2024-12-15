use glib::{RustClosure, SpawnFlags};
use gtk4::{gio::spawn_blocking, Align, Box, Button, Entry, Label, Orientation};
use libadwaita::{prelude::*, ApplicationWindow, HeaderBar, Window};
use vte4::{PtyFlags, Regex, Terminal as Vte, TerminalExt, TerminalExtManual};

use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, Write}, time::Duration,
};

use crate::application::IvyApplication;

pub fn spawn_new_tmux_modal(parent: &ApplicationWindow) {
    let app = parent.application().unwrap();

    let dialog = Window::builder()
        .application(&app)
        .title("Attach new Tmux session")
        .modal(true)
        .transient_for(parent)
        .build();

    let header_bar = HeaderBar::new();
    let content = Box::new(Orientation::Vertical, 5);

    // Tmux session input
    let session_label = Label::new(Some("Tmux session:"));
    let session_input = Entry::new();
    content.append(&session_label);
    content.append(&session_input);

    // SSH input
    let ssh_label = Label::new(Some("SSH command:"));
    let ssh_input = Entry::new();
    content.append(&ssh_label);
    content.append(&ssh_input);

    // Button
    let button = Button::builder().label("Attach").build();
    content.append(&button);

    let window_box = Box::new(Orientation::Vertical, 0);
    window_box.append(&header_bar);
    window_box.append(&content);
    dialog.set_content(Some(&window_box));

    button.connect_clicked(glib::clone!(
        #[weak]
        dialog,
        #[weak]
        parent,
        move |_| {
            let tmux_session = session_input.text();
            let ssh_target = ssh_input.text();

            dialog.close();

            spawn_ssh_login(&parent, &tmux_session, &ssh_target);
            // if let Some(app) = app {
            //     let app: IvyApplication = app.downcast().unwrap();
            //     let ssh_target = if ssh_target.is_empty() {
            //         None
            //     } else {
            //         Some(ssh_target.as_str())
            //     };
            //     app.new_window(Some(tmux_session.as_str()), ssh_target);
            // }
        }
    ));

    dialog.present();
}

pub fn spawn_ssh_login(parent: &ApplicationWindow, tmux_session: &str, ssh_target: &str) {
    let app: IvyApplication = parent.application().unwrap().downcast().unwrap();

    let dialog = Window::builder()
        .application(&app)
        .title("Attach new Tmux session")
        .modal(true)
        .transient_for(parent)
        .build();

    let window_box = Box::new(Orientation::Vertical, 0);
    let header_bar = HeaderBar::new();
    window_box.append(&header_bar);

    let config = app.get_terminal_config();

    let vte = Vte::builder()
        .vexpand(true)
        .hexpand(true)
        .font_desc(config.font.as_ref())
        .audible_bell(config.terminal_bell)
        .scrollback_lines(config.scrollback_lines)
        .allow_hyperlink(true)
        .build();

    vte.connect_child_exited(glib::clone!(
        #[weak]
        dialog,
        move |_, _| {
            dialog.close();
        }
    ));

    // Spawn terminal
    let pty_flags = PtyFlags::DEFAULT;
    let spawn_flags = SpawnFlags::DEFAULT;

    // Set shell
    let mut argv: Vec<&str> = Vec::new();
    argv.push("/bin/sh");
    // argv.push("-c");

    let stdin_fifo = "/tmp/ivyterm/ssh_stdin";
    let stdout_fifo = "/tmp/ivyterm/ssh_stdout";

    // Execute SSH and redirect STDIN/STDOUT
    let tmux_command = format!("tmux -2 -C new-session -A -s {}", tmux_session);
    let ssh_command = format!(
        "nohup ssh {} '{}' < {} > {}\n",
        ssh_target, tmux_command, stdin_fifo, stdout_fifo
    );
    // argv.push(ssh_command.as_str());

    // Set environment variables
    let envv = std::env::vars();
    let envv: Vec<String> = envv.map(|(key, val)| key + "=" + &val).collect();
    let envv: Vec<&str> = envv.iter().map(|s| s.as_str()).collect();

    vte.spawn_async(
        pty_flags,
        None,
        &argv,
        &envv,
        spawn_flags,
        || {},
        -1,
        gtk4::gio::Cancellable::NONE,
        glib::clone!(
            #[weak]
            vte,
            move |_result| {
                vte.grab_focus();
                vte.feed_child(ssh_command.as_bytes());
            }
        ),
    );

    window_box.append(&vte);
    dialog.set_content(Some(&window_box));

    glib::spawn_future_local(glib::clone!(
        #[weak]
        vte,
        async move {
            spawn_blocking(move || {
                let five_seconds = Duration::from_secs(5);
                std::thread::sleep(five_seconds);
                true
            })
            .await
            .expect("Task needs to finish successfully.");

            let ctrlz: u8 = 26;
            // vte.feed_child("lala123xxx".as_bytes());
            vte.feed_child(&[ctrlz]);
            vte.feed_child("bg\nexit\n".as_bytes());
        }
    ));

    dialog.present();
}
