use std::{cell::RefCell, rc::Rc};

use gtk4::{FontDialog, FontDialogButton};
use libadwaita::{prelude::*, PreferencesGroup, PreferencesPage};

use crate::config::GlobalConfig;

use super::{create_color_button, create_setting_row};

pub fn create_general_page(config: &Rc<RefCell<GlobalConfig>>) -> PreferencesPage {
    // Page 1: Color and Font dialogs
    let page = PreferencesPage::builder().title("General").build();

    let terminal_prefs = create_terminal_prefs(config);
    page.add(&terminal_prefs);

    // Color scheme
    let standard_colors = create_standard_colors(config);
    page.add(&standard_colors);
    let bright_colors = create_bright_colors(config);
    page.add(&bright_colors);

    page
}

fn create_terminal_prefs(config: &Rc<RefCell<GlobalConfig>>) -> PreferencesGroup {
    let borrowed = config.borrow();

    // Font Dialog
    let main_font = FontDialogButton::new(Some(FontDialog::new()));
    main_font.set_font_desc(borrowed.terminal.font.as_ref());
    main_font.connect_font_desc_notify(glib::clone!(
        #[strong]
        config,
        move |button| {
            let mut borrowed = config.borrow_mut();
            borrowed.terminal.font = button.font_desc().unwrap().into();
        }
    ));

    // Foreground color
    let foreground_color = create_color_button(&borrowed.terminal.foreground);
    foreground_color.connect_rgba_notify(glib::clone!(
        #[strong]
        config,
        move |button| {
            let mut borrowed = config.borrow_mut();
            borrowed.terminal.foreground = button.rgba().into();
        }
    ));

    // Background
    let background_color = create_color_button(&borrowed.terminal.background);
    background_color.connect_rgba_notify(glib::clone!(
        #[strong]
        config,
        move |button| {
            let mut borrowed = config.borrow_mut();
            borrowed.terminal.background = button.rgba().into();
        }
    ));

    // Build the page itself
    let terminal_font_color = PreferencesGroup::builder()
        .title("Terminal font and colors")
        .build();

    create_setting_row(&terminal_font_color, "Terminal font", main_font);
    create_setting_row(&terminal_font_color, "Foreground color", foreground_color);
    create_setting_row(&terminal_font_color, "Background color", background_color);

    terminal_font_color
}

fn create_standard_colors(config: &Rc<RefCell<GlobalConfig>>) -> PreferencesGroup {
    // Build the page itself
    let standard_colors = PreferencesGroup::builder()
        .title("Standard colors")
        .build();

    let borrowed = config.borrow();

    for (idx, _) in borrowed.terminal.standard_colors.iter().enumerate() {
        let button = create_color_button(&borrowed.terminal.standard_colors[idx]);
        button.connect_rgba_notify(glib::clone!(
            #[strong]
            config,
            move |button| {
                let mut borrowed = config.borrow_mut();
                borrowed.terminal.standard_colors[idx] = button.rgba().into();
            }
        ));

        let name = format!("Standard color {}", idx);
        create_setting_row(&standard_colors, &name, button);
    }

    standard_colors
}

fn create_bright_colors(config: &Rc<RefCell<GlobalConfig>>) -> PreferencesGroup {
    // Build the page itself
    let bright_colors = PreferencesGroup::builder()
        .title("Bright colors")
        .build();

    let borrowed = config.borrow();

    for (idx, _) in borrowed.terminal.bright_colors.iter().enumerate() {
        let button = create_color_button(&borrowed.terminal.bright_colors[idx]);
        button.connect_rgba_notify(glib::clone!(
            #[strong]
            config,
            move |button| {
                let mut borrowed = config.borrow_mut();
                borrowed.terminal.bright_colors[idx] = button.rgba().into();
            }
        ));

        let name = format!("Bright color {}", idx);
        create_setting_row(&bright_colors, &name, button);
    }

    bright_colors
}
