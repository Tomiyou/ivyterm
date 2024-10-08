use glib::SpawnFlags;
use vte4::{PtyFlags, Terminal, TerminalExtManual};

pub fn create_terminal() -> Terminal {
    let vte = Terminal::builder().vexpand(true).hexpand(true).build();

    let pty_flags = PtyFlags::DEFAULT;
    let argv = ["/bin/bash"];
    let envv = [];
    let spawn_flags = SpawnFlags::DEFAULT;

    vte.spawn_async(
        pty_flags,
        None,
        &argv,
        &envv,
        spawn_flags,
        || {
            println!("Lmao its me Mario");
        },
        -1,
        gtk::gio::Cancellable::NONE,
        |_| println!("Some callback?"),
    );

    vte
}
