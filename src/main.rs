use std::sync::atomic::AtomicU32;

use gtk4::gdk::Display;
use gtk4::gio::ApplicationFlags;
use gtk4::{
    Align, Box as GtkBox, Button, CssProvider, Orientation, PackType, WindowControls, WindowHandle,
};
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow, TabBar, TabView};

use global_state::{show_settings_window, APPLICATION_TITLE, INITIAL_HEIGHT, INITIAL_WIDTH};
use tmux::attach_tmux;
use toplevel::create_tab;

mod error;
mod global_state;
mod keyboard;
mod paned;
mod terminal;
mod tmux;
mod toplevel;

static GLOBAL_TAB_ID: AtomicU32 = AtomicU32::new(0);

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

fn create_window(app: &Application, tmux_session: Option<&str>) {
    // Create a new window
    let window = ApplicationWindow::builder()
        .application(app)
        .title(APPLICATION_TITLE)
        .default_width(INITIAL_WIDTH)
        .default_height(INITIAL_HEIGHT)
        .build();

    println!("create_window called");
    if let Some(session_name) = tmux_session {
        println!("Starting TMUX");
        attach_tmux(session_name);
    }

    // Window content box holds title bar and panes
    let window_box = GtkBox::new(Orientation::Vertical, 0);

    // View stack holds all panes
    let tab_view = TabView::new();

    // Close the tab_view when 0 tabs remain
    let _window = window.clone();
    tab_view.connect_close_page(move |tab_view, _page| {
        if tab_view.n_pages() < 2 {
            _window.close();
        }
        false
    });

    // Create initial Tab
    create_tab(&tab_view);

    // Terminal settings
    let settings_button = Button::with_label("Settings");
    let _app = app.clone();
    settings_button.connect_clicked(move |_button| {
        show_settings_window(_app.clone());
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
    let header_box = GtkBox::new(Orientation::Horizontal, 0);
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

    window.set_content(Some(&window_box));
    window.present();
}

fn main() -> glib::ExitCode {
    let application = Application::builder()
        .application_id("com.tomiyou.ivyTerm")
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    application.connect_command_line(move |app, cli| {
        let mut tmux_mode: Option<&str> = None;

        let args = cli.arguments();
        let mut iterator = args.iter();
        while let Some(arg) = iterator.next() {
            if arg.eq_ignore_ascii_case("--tmux") {
                // println!("Launching as tmux");
                tmux_mode = if let Some(ssh) = iterator.next() {
                    println!("Attaching TMUX remote on {:?}", ssh);
                    ssh.to_str()
                } else {
                    println!("Attaching TMUX locally");
                    Some("")
                };
            }
        }

        // Handle this better
        create_window(app, tmux_mode);

        0
    });

    application.connect_startup(|_| load_css());
    application.run()
}
