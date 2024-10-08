use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use libadwaita::prelude::*;
use libadwaita::{glib::signal::Propagation, Application, ApplicationWindow, TabBar, TabView};
use glib::property::PropertyGet;
use gtk4::gdk::{Display, ModifierType};
use gtk4::{
    Align, Box, Button, CssProvider, EventControllerKey, Orientation, PackType, WindowControls,
    WindowHandle,
};

use global_state::{GlobalSettings, APPLICATION_TITLE, INITIAL_HEIGHT, INITIAL_WIDTH};

mod global_state;
mod keyboard;
mod mux;

static GLOBAL_TAB_ID: AtomicU32 = AtomicU32::new(0);
static TAB_COUNT: AtomicU32 = AtomicU32::new(0);
static GLOBAL_TERMINAL_ID: AtomicU32 = AtomicU32::new(0);

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("style.css"));

    // Add the provider to the default screen
    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn create_tab(tab_view: &TabView) {
    let tab_id = GLOBAL_TAB_ID.fetch_add(1, Ordering::Relaxed);
    let tab = mux::Tab::new(tab_id);

    // Add pane as a page
    let page = tab_view.append(&tab);
    let text = format!("Terminal {}", tab_id);
    page.set_title(&text);
    tab_view.set_selected_page(&page);
}

fn main() -> glib::ExitCode {
    let application = Application::builder()
        .application_id("com.tomiyou.Windbreeze")
        .build();
    let application = Rc::new(application);

    application.connect_startup(|_| load_css());

    let application_rc = application.clone();
    application.connect_activate(move |app| {
        // Create a new window
        let window = ApplicationWindow::builder()
            .application(app)
            .title(APPLICATION_TITLE)
            .default_width(INITIAL_WIDTH)
            .default_height(INITIAL_HEIGHT)
            .build();
        let window = Rc::new(window);

        // Initialize global settings
        let global_settings = GlobalSettings::new(application_rc.clone());

        // Window content box holds title bar and panes
        let window_box = Box::new(Orientation::Vertical, 0);

        // View stack holds all panes
        let tab_view = TabView::new();
        create_tab(&tab_view);

        // Terminal settings
        let settings_button = Button::with_label("Settings");
        settings_button.connect_clicked(move |_button| {
            global_settings.show_settings_window();
        });

        // View switcher for switching between open tabs
        let tab_bar = TabBar::builder()
            .css_classes(vec!["inline"])
            .margin_top(0)
            .margin_bottom(0)
            .halign(Align::Fill)
            .hexpand(true)
            .autohide(false)
            .can_focus(false)
            .expand_tabs(false)
            .view(&tab_view)
            .end_action_widget(&settings_button)
            .build();

        // Header box holding tabs and window controls
        let left_window_controls = WindowControls::new(PackType::Start);
        let right_window_controls = WindowControls::new(PackType::End);
        let header_box = Box::new(Orientation::Horizontal, 0);
        header_box.append(&left_window_controls);
        header_box.append(&tab_bar);
        header_box.append(&right_window_controls);

        // Header bar
        let window_handle = WindowHandle::builder()
            .child(&header_box)
            .css_classes(vec!["header-margin"])
            .build();

        window_box.append(&window_handle);
        window_box.append(&tab_view);

        // Keyboard handling
        let key_ctl_window = window.clone();
        let eventctl = {
            let eventctl = EventControllerKey::new();
            eventctl.connect_key_pressed(move |eventctl, keyval, keycode, state| {
                // let frame_time = Duration::new(0, 1_000_000_000 / 1);

                // if let Some(char) = keyval.to_unicode() {
                //     println!("Pressed button {}", char);
                //     let name = format!("Page {}", char);
                //     // stack.set_visible_child_name(&name);
                // } else {
                // }

                // let modifier = ModifierType::CONTROL_MASK;
                // let modifier = modifier.union(ModifierType::SHIFT_MASK);
                if state.contains(ModifierType::CONTROL_MASK)
                    && state.contains(ModifierType::SHIFT_MASK)
                {
                    if keycode == 28 {
                        create_tab(&tab_view);
                        return Propagation::Stop;
                    } else if keycode == 25 {
                        let tab_count = TAB_COUNT.fetch_sub(1, Ordering::Relaxed);
                        if tab_count == 1 {
                            key_ctl_window.close();
                            println!("EXITING");
                            return Propagation::Stop;
                        }

                        let page = tab_view.selected_page().unwrap();
                        tab_view.close_page(&page);
                        return Propagation::Stop;
                    }
                }

                // println!("No match");
                Propagation::Proceed
            });
            eventctl
        };
        window.add_controller(eventctl);

        window.set_content(Some(&window_box));
        window.present();
    });

    application.run()
}
