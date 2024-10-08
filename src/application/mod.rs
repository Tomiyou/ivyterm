mod config;
mod imp;

use glib::Object;
use gtk4::gdk::Display;
use gtk4::CssProvider;
use libadwaita::subclass::prelude::*;
use libadwaita::{gio, glib};
use vte4::{ApplicationExt, Cast, GtkApplicationExt, GtkWindowExt};

use crate::settings::show_preferences_window;
use crate::tmux::attach_tmux;
use crate::window::IvyWindow;

glib::wrapper! {
    pub struct IvyApplication(ObjectSubclass<imp::IvyApplicationPriv>)
        @extends libadwaita::Application, gtk4::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl IvyApplication {
    pub fn new() -> Self {
        let app: IvyApplication = Object::builder().build();
        app.set_application_id(Some("com.tomiyou.ivyTerm"));
        app
    }

    pub fn init_css_provider(&self) {
        let css_provider = load_css();
        self.imp().css_provider.replace(Some(css_provider));
        println!("Css provider set!");
    }

    pub fn init_keybindings(&self) {
        let imp = self.imp();
        let mut config = imp.config.borrow_mut();
        let mut parsed_keybindings = config.keybindings.init();

        let mut keybindings = imp.keybindings.borrow_mut();
        keybindings.append(&mut parsed_keybindings)
    }

    pub fn new_window(&self, tmux_session: Option<&str>) {
        let imp = self.imp();
        let binding = imp.css_provider.borrow();
        let css_provider = binding.as_ref().unwrap();
        let window = IvyWindow::new(self, css_provider);

        if let Some(session_name) = tmux_session {
            println!("Starting TMUX");
            let tmux = attach_tmux(session_name, &window).unwrap();
            window.init_tmux(tmux);
        } else {
            // Create initial Tab
            window.new_tab(None);
        }

        window.present();
    }

    fn reload_css_colors(&self) {
        let config = self.imp().config.borrow();
        let background_hex = config.background.to_hex();

        // Update CSS colors (background and separator)
        let binding = self.imp().css_provider.borrow();
        let css_provider = binding.as_ref().unwrap();
        let new_css = BASE_CSS.replace("#000000", &background_hex);
        css_provider.load_from_data(&new_css);

        self.refresh_terminals();
    }

    pub fn show_settings(&self) {
        show_preferences_window(self);
    }

    fn refresh_terminals(&self) {
        let config = self.imp().config.borrow();
        let (font_desc, main_colors, palette_colors, scrollback_lines) =
            config.get_terminal_config();

        // Refresh terminals to respect the new colors
        for window in self.windows() {
            if let Ok(window) = window.downcast::<IvyWindow>() {
                window.update_terminal_config(
                    &font_desc,
                    main_colors,
                    palette_colors,
                    scrollback_lines,
                );
            }
        }
    }
}

static BASE_CSS: &str = include_str!("style.css");

fn load_css() -> CssProvider {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_data(BASE_CSS);

    // Add the provider to the default screen
    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    provider
}
