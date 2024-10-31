use gtk4::{FontDialog, FontDialogButton};
use libadwaita::{prelude::*, PreferencesGroup, PreferencesPage};

use crate::{application::IvyApplication, config::GlobalConfig};

use super::{create_color_button, create_setting_row};

fn create_terminal_prefs(app: &IvyApplication, data: &GlobalConfig) -> PreferencesGroup {
    let app = app.clone();

    // Font Dialog
    let main_font = FontDialogButton::new(Some(FontDialog::new()));
    main_font.set_font_desc(data.font.as_ref());
    main_font.connect_font_desc_notify(glib::clone!(
        #[weak]
        app,
        move |button| {
            let font_description = button.font_desc().unwrap();
            app.update_font(font_description);
        }
    ));

    // Foreground color
    let foreground_color = create_color_button(&data.foreground);
    foreground_color.connect_rgba_notify(glib::clone!(
        #[weak]
        app,
        move |button| {
            let rgba = button.rgba();
            app.update_foreground_color(rgba)
        }
    ));

    // Background
    let background_color = create_color_button(&data.background);
    background_color.connect_rgba_notify(move |button| {
        let rgba = button.rgba();
        app.update_background_color(rgba)
    });

    // Build the page itself
    let terminal_font_color = PreferencesGroup::builder()
        .title("Terminal font and colors")
        .build();

    create_setting_row(&terminal_font_color, "Terminal font", main_font);
    create_setting_row(&terminal_font_color, "Foreground color", foreground_color);
    create_setting_row(&terminal_font_color, "Background color", background_color);

    terminal_font_color
}

pub fn create_general_page(app: &IvyApplication, data: &GlobalConfig) -> PreferencesPage {
    // Page 1: Color and Font dialogs
    let page = PreferencesPage::builder().title("General").build();

    let terminal_prefs = create_terminal_prefs(app, data);
    page.add(&terminal_prefs);

    page
}
