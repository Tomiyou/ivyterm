use std::{cell::RefCell, rc::Rc};

use gtk4::{FontDialog, FontDialogButton};
use libadwaita::{prelude::*, PreferencesGroup, PreferencesPage};

use crate::config::GlobalConfig;

use super::{create_color_button, create_setting_row};

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

pub fn create_general_page(config: &Rc<RefCell<GlobalConfig>>) -> PreferencesPage {
    // Page 1: Color and Font dialogs
    let page = PreferencesPage::builder().title("General").build();

    let terminal_prefs = create_terminal_prefs(config);
    page.add(&terminal_prefs);

    page
}
