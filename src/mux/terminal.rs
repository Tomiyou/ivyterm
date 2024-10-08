use glib::SpawnFlags;
use vte4::{PtyFlags, Terminal, TerminalExt, TerminalExtManual, WidgetExt};

use crate::global_state::GLOBAL_SETTINGS;

pub fn create_terminal() -> Terminal {
    // Get terminal font
    let font_desc = {
        let reader = GLOBAL_SETTINGS.read().unwrap();
        reader.font_desc.clone()
    };

    let terminal = Terminal::builder()
        .vexpand(true)
        .hexpand(true)
        .font_desc(&font_desc)
        .build();

    terminal.connect_child_exited(|terminal, exit_code| {
        println!("Exited!");
    });

    // Spawn terminal
    let pty_flags = PtyFlags::DEFAULT;
    let argv = ["/bin/bash"];
    let envv = [];
    let spawn_flags = SpawnFlags::DEFAULT;

    let _terminal = terminal.clone();
    terminal.spawn_async(
        pty_flags,
        None,
        &argv,
        &envv,
        spawn_flags,
        || {
            println!("Lmao its me Mario");
        },
        -1,
        gtk4::gio::Cancellable::NONE,
        move |_result| {
            println!("Some callback?");
            _terminal.grab_focus();
        },
    );

    terminal
}
