use gtk4::gdk::{Key, ModifierType};

pub enum Keybinding {
    TabNew,
    TabClose,
    PaneSplit(bool),
    PaneClose,
}

pub fn matches_keybinding(
    keyval: Key,
    keycode: u32,
    state: ModifierType,
    keybinding: Keybinding,
) -> bool {
    if state.contains(ModifierType::CONTROL_MASK) && state.contains(ModifierType::SHIFT_MASK) {
        let matches = match (keybinding, keycode) {
            (Keybinding::TabNew, 28) => {
                println!("Keybinding::TabNew");
                true
            }
            (Keybinding::TabClose, 25) => {
                println!("Keybinding::TabClose");
                true
            }
            (Keybinding::PaneSplit(true), 32) => {
                println!("Keybinding::PaneSplit");
                true
            }
            (Keybinding::PaneSplit(false), 46) => {
                println!("Keybinding::PaneSplit");
                true
            }
            (Keybinding::PaneClose, 26) => {
                println!("Keybinding::PaneClose");
                true
            }
            _ => false,
        };

        if matches {
            return true;
        }
    }

    false
}

// {
//     if keycode == 28 {
//         create_tab(&tab_view);
//         return Propagation::Stop;
//     } else if keycode == 25 {
//         let tab_count = TAB_COUNT.fetch_sub(1, Ordering::Relaxed);
//         if tab_count == 1 {
//             key_ctl_window.close();
//             println!("EXITING");
//             return Propagation::Stop;
//         }

//         let page = tab_view.selected_page().unwrap();
//         tab_view.close_page(&page);
//         return Propagation::Stop;
//     }
// }
