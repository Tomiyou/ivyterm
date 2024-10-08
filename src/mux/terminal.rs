use glib::SpawnFlags;
use vte4::{PtyFlags, Terminal, TerminalExt, TerminalExtManual, WidgetExt};

pub fn create_terminal() -> Terminal {
    let terminal = Terminal::builder().vexpand(true).hexpand(true).build();

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

    terminal.connect_child_exited(|terminal, exit_code| {
        println!("Exited!");
    });

    // Change terminal font
    // terminal.set_font_desc(font_desc)

    terminal
}
