use gtk4::{Align, Box, ColorDialog, ColorDialogButton, FontDialog, FontDialogButton, Label, Orientation, Widget};
use libadwaita::{prelude::*, PreferencesGroup, PreferencesPage, PreferencesRow};

use crate::application::IvyApplication;

fn create_setting_row(name: &str, child: impl IsA<Widget>) -> PreferencesRow {
    child.set_halign(Align::End);

    let label = Label::builder()
        .hexpand(true)
        .halign(Align::Start)
        .label(name)
        .build();
    
    let row_box = Box::new(Orientation::Horizontal, 0);
    row_box.append(&label);
    row_box.append(&child);

    let row = PreferencesRow::builder()
        .title(name)
        .child(&row_box)
        .css_classes(["setting_row"])
        .build();

    row
}

fn create_terminal_prefs(app: &IvyApplication) -> PreferencesGroup {
    let app = app.clone();

    // Font Dialog
    let font_dialog = FontDialog::new();
    let font_dialog_button = FontDialogButton::new(Some(font_dialog));
    font_dialog_button.connect_font_desc_notify(glib::clone!(
        #[weak]
        app,
        move |button| {
            let font_description = button.font_desc().unwrap();
            app.update_font(font_description);
        }
    ));
    let terminal_font_row = create_setting_row("Terminal font", font_dialog_button);

    // Foreground color
    let foreground_color_dialog = ColorDialog::new();
    let foreground_color_button = ColorDialogButton::new(Some(foreground_color_dialog));
    foreground_color_button.connect_rgba_notify(glib::clone!(
        #[weak]
        app,
        move |button| {
            let rgba = button.rgba();
            app.update_foreground_color(rgba)
        }
    ));
    let foreground_color_row = create_setting_row("Foreground color", foreground_color_button);

    // Background
    let background_color_dialog = ColorDialog::new();
    let background_color_button = ColorDialogButton::new(Some(background_color_dialog));
    background_color_button.connect_rgba_notify(move |button| {
        let rgba = button.rgba();
        app.update_background_color(rgba)
    });
    let background_color_row = create_setting_row("Background color", background_color_button);

    // Build the page itself
    let terminal_font_color = PreferencesGroup::builder()
        .title("Terminal font and colors")
        .build();

    terminal_font_color.add(&terminal_font_row);
    terminal_font_color.add(&foreground_color_row);
    terminal_font_color.add(&background_color_row);

    terminal_font_color
}

pub fn create_general_page(app: &IvyApplication) -> PreferencesPage {
    // Page 1: Color and Font dialogs
    let page = PreferencesPage::builder().title("General").build();

    let terminal_prefs = create_terminal_prefs(app);
    page.add(&terminal_prefs);

    page
}
