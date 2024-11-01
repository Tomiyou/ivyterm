use std::{cell::RefCell, rc::Rc};

use libadwaita::{prelude::*, PreferencesGroup, PreferencesPage};

use crate::config::GlobalConfig;

use super::{create_color_button, create_setting_row};

fn create_appearance_prefs(config: &Rc<RefCell<GlobalConfig>>) -> PreferencesGroup {
    let borrowed = config.borrow();

    // Foreground color
    let window_color = create_color_button(&borrowed.tmux.window_color);
    window_color.connect_rgba_notify(glib::clone!(
        #[strong]
        config,
        move |button| {
            let mut borrowed = config.borrow_mut();
            borrowed.tmux.window_color = button.rgba().into();
        }
    ));

    // Build the page itself
    let tmux_colors = PreferencesGroup::builder().title("Color").build();

    create_setting_row(&tmux_colors, "Tmux window color", window_color);

    tmux_colors
}

pub fn create_tmux_page(config: &Rc<RefCell<GlobalConfig>>) -> PreferencesPage {
    // Page 2: Tmux settings
    let page = PreferencesPage::builder().title("Tmux").build();

    let appearance_prefs = create_appearance_prefs(config);
    page.add(&appearance_prefs);

    page
}
