use glib::SpawnFlags;
use gtk4::{gdk::RGBA, Paned};
use libadwaita::Bin;
use vte4::{BuildableExt, Cast, PtyFlags, Terminal, TerminalExt, TerminalExtManual, WidgetExt};

use crate::{global_state::GLOBAL_SETTINGS, mux::{close_tab, pane::close_pane}};

fn default_colors() -> (RGBA, RGBA) {
    let foreground = RGBA::new(1.0, 1.0, 1.0, 1.0);
    let background = RGBA::new(0.0, 0.0, 0.0, 1.0);

    (foreground, background)
}

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
        terminal.unrealize();

        let parent = terminal.parent().unwrap();
        if let Ok(paned) = parent.clone().downcast::<Paned>() {
            close_pane(paned);
        } else if let Ok(bin) = parent.downcast::<Bin>() {
            close_tab(bin);
        } else {
            panic!("Parent is neither Bin nor Paned");
        }
    });

    let (foreground, background) = default_colors();
    terminal.set_colors(Some(&foreground), Some(&background), &[]);

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