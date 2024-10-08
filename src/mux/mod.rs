mod pane;
mod terminal;

use libadwaita::glib::signal::Propagation;
use libadwaita::{prelude::*, Bin};
use gtk4::{EventControllerKey, Orientation, Paned, ScrolledWindow};

use crate::keyboard::matches_keybinding;

use self::pane::Pane;
use self::terminal::create_terminal;

pub struct Tab {
    // id: u32,
    // panes: Arc<RwLock<HashMap<u32, Arc<RwLock<Terminal>>>>>,
    // child: Paned,
}

impl Tab {
    pub fn new(tab_id: u32) -> Bin {
        let terminal = create_terminal();

        let bin = Bin::builder()
            .child(&terminal)
            .vexpand(true)
            .hexpand(true)
            .focusable(true)
            .build();

        let eventctl = EventControllerKey::new();
        eventctl.connect_key_pressed(move |eventctl, keyval, keycode, state| {
            // Handle terminal splits
            let bin = eventctl.widget();
            let bin = bin.downcast::<Bin>().unwrap();

            // println!("Bin keycode {}\n", keycode);

            if matches_keybinding(
                keyval,
                keycode,
                state,
                crate::keyboard::Keybinding::PaneSplit(true),
            ) {
                split(bin, Orientation::Vertical);
                return Propagation::Stop;
            }
            if matches_keybinding(
                keyval,
                keycode,
                state,
                crate::keyboard::Keybinding::PaneSplit(false),
            ) {
                split(bin, Orientation::Horizontal);
                return Propagation::Stop;
            }

            Propagation::Proceed
        });
        bin.add_controller(eventctl);

        bin
    }
}

fn split(bin: Bin, orientation: Orientation) {
    let old_terminal = bin.child().unwrap();

    // If GTK focus is wrong, don't split the pane
    if !old_terminal.clone().downcast::<Paned>().is_err() {
        return;
    }

    let new_terminal = create_terminal();
    let none: Option<&Bin> = None;

    bin.set_child(none);
    let new_paned = Pane::new(orientation, old_terminal, new_terminal);
    bin.set_child(Some(&new_paned));

    println!("New PANE {:?}", new_paned.as_ptr())
}

pub fn close_tab(bin: Bin) {
    println!("Closing tab!");
    bin.unrealize();
}
