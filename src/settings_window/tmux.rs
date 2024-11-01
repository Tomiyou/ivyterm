use libadwaita::{prelude::*, PreferencesGroup, PreferencesPage};

use crate::{application::IvyApplication, config::GlobalConfig};

use super::{create_color_button, create_setting_row};

fn create_appearance_prefs(app: &IvyApplication, data: &GlobalConfig) -> PreferencesGroup {
    let app = app.clone();

    // Foreground color
    let window_color = create_color_button(&data.tmux.window_color);
    window_color.connect_rgba_notify(glib::clone!(
        #[weak]
        app,
        move |button| {
            let rgba = button.rgba();
            app.update_foreground_color(rgba)
        }
    ));

    // Build the page itself
    let tmux_colors = PreferencesGroup::builder()
        .title("Color")
        .build();

    create_setting_row(&tmux_colors, "Tmux window color", window_color);

    tmux_colors
}

pub fn create_tmux_page(app: &IvyApplication, data: &GlobalConfig) -> PreferencesPage {
    // Page 2: Tmux settings
    let page = PreferencesPage::builder().title("Tmux").build();

    let appearance_prefs = create_appearance_prefs(app, data);
    page.add(&appearance_prefs);

    page
}
