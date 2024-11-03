mod config;
mod imp;

use glib::Object;
use gtk4::gdk::Display;
use gtk4::CssProvider;
use libadwaita::subclass::prelude::*;
use libadwaita::{gio, glib, PreferencesWindow};
use log::debug;
// TODO: This should be libadwaita::prelude::*
use vte4::{ApplicationExt, Cast, GtkApplicationExt, GtkWindowExt};

use crate::normal_widgets::IvyNormalWindow;
use crate::settings_window::spawn_preferences_window;
use crate::tmux_widgets::IvyTmuxWindow;

const APPLICATION_ID: &str = "com.tomiyou.ivyTerm";

static BASE_CSS: &str = include_str!("style.css");

glib::wrapper! {
    pub struct IvyApplication(ObjectSubclass<imp::IvyApplicationPriv>)
        @extends libadwaita::Application, gtk4::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl IvyApplication {
    pub fn new() -> Self {
        let app: IvyApplication = Object::builder().build();
        app.set_application_id(Some(APPLICATION_ID));
        app
    }

    pub fn init_css_provider(&self) {
        // Load the CSS file and add it to the provider
        let css_provider = CssProvider::new();
        self.parse_css(&css_provider);

        // Add the provider to the default screen
        gtk4::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        self.imp().css_provider.replace(Some(css_provider));
        debug!("Css provider set!");
    }

    pub fn init_keybindings(&self) {
        let imp = self.imp();
        let mut config = imp.config.borrow_mut();
        let mut parsed_keybindings = config.keybindings.init();

        let mut keybindings = imp.keybindings.borrow_mut();
        keybindings.append(&mut parsed_keybindings)
    }

    pub fn new_window(&self, tmux_session: Option<&str>, ssh_target: Option<&str>) {
        let imp = self.imp();
        let binding = imp.css_provider.borrow();
        let css_provider = binding.as_ref().unwrap();

        if let Some(session_name) = tmux_session {
            let window = IvyTmuxWindow::new(self, css_provider, session_name, ssh_target);
            window.present();
        } else {
            // Create initial Tab
            let window = IvyNormalWindow::new(self, css_provider);
            window.present();
        };
    }

    fn reload_css(&self) {
        // Update CSS colors (background and separator)
        let binding = self.imp().css_provider.borrow();
        let css_provider = binding.as_ref().unwrap();
        self.parse_css(&css_provider);

        self.refresh_terminals();
    }

    pub fn show_settings(&self) {
        // If a Settings window is already open, simply bring it to the front
        for window in self.windows() {
            if let Ok(window) = window.downcast::<PreferencesWindow>() {
                debug!("Presenting an already open Settings window");
                window.present();
                return;
            }
        }

        let config = self.imp().config.borrow().clone();
        spawn_preferences_window(self, config);
    }

    fn refresh_terminals(&self) {
        let config = self.get_terminal_config();

        // Refresh terminals to respect the new colors
        for window in self.windows() {
            // Handle non-Tmux windows
            let window = match window.downcast::<IvyNormalWindow>() {
                Ok(window) => {
                    window.update_terminal_config(&config);
                    continue;
                }
                Err(window) => window,
            };

            // Handle Tmux windows
            if let Ok(window) = window.downcast::<IvyTmuxWindow>() {
                window.update_terminal_config(&config);
            }
        }
    }

    #[inline]
    fn parse_css(&self, css_provider: &CssProvider) {
        let config = self.imp().config.borrow();
        let background_hex = config.terminal.background.to_hex();
        let tmux_window_hex = config.tmux.window_color.to_hex();
        let split_handle_hex = config.terminal.split_handle_color.to_hex();

        let css = BASE_CSS
            .replace("#f0f0f0", &split_handle_hex)
            .replace("#000000", &background_hex)
            .replace("#420a42", &tmux_window_hex);

        css_provider.load_from_data(&css);
    }
}
