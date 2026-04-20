use application::IvyApplication;
use libadwaita::{gio, glib};
use libadwaita::prelude::*;

mod application;
mod config;
mod helpers;
mod keyboard;
mod modals;
mod normal_widgets;
mod settings_window;
mod ssh;
mod tmux_api;
mod tmux_widgets;

fn main() -> glib::ExitCode {
    // Handle --help before GTK initialization
    let args: Vec<String> = std::env::args().collect();
    if args[1..].iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: ivyterm [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -t, --tmux <SESSION>  Attach to a tmux session");
        println!("  -s, --ssh <HOST>      SSH host (e.g., user@host)");
        println!("  -h, --help            Show this help message");
        std::process::exit(0);
    }

    env_logger::init();

    let application = IvyApplication::new();
    application.set_flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE);

    // Initialize IvyApplication
    application.connect_startup(|app| {
        app.init_css_provider();
        app.init_keybindings();
    });

    application.connect_command_line(move |app, cmdline| {
        let args: Vec<String> = cmdline
            .arguments()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();

        let mut tmux_session = None;
        let mut ssh_host = None;
        let mut i = 1; // skip program name

        while i < args.len() {
            match args[i].as_str() {
                "--tmux" | "-t" => {
                    i += 1;
                    if i >= args.len() || args[i].starts_with('-') {
                        eprintln!("Error: {} requires a value", args[i - 1]);
                        return 1;
                    }
                    tmux_session = Some(args[i].clone());
                }
                "--ssh" | "-s" => {
                    i += 1;
                    if i >= args.len() || args[i].starts_with('-') {
                        eprintln!("Error: {} requires a value", args[i - 1]);
                        return 1;
                    }
                    ssh_host = Some(args[i].clone());
                }
                arg => {
                    eprintln!("Error: unknown argument '{}'", arg);
                    return 1;
                }
            }
            i += 1;
        }

        if ssh_host.is_some() && tmux_session.is_none() {
            eprintln!("Error: --ssh requires --tmux");
            return 1;
        }

        if let Some(session) = tmux_session {
            let ssh_target = ssh_host.as_deref().map(|h| (h, ""));
            app.new_tmux_window(&session, ssh_target);
        } else {
            app.new_normal_window();
        }

        0
    });
    application.run()
}
