mod config;
mod imp;

use glib::Object;
use gtk4::gdk::{Display, RGBA};
use gtk4::CssProvider;
use libadwaita::subclass::prelude::*;
use libadwaita::{gio, glib};
use vte4::{ApplicationExt, Cast, GtkApplicationExt, GtkWindowExt};

use crate::tmux::attach_tmux;
use crate::window::IvyWindow;

pub use config::APPLICATION_TITLE;
pub use config::INITIAL_HEIGHT;
pub use config::INITIAL_WIDTH;
pub use config::SPLIT_HANDLE_WIDTH;

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
        let [_foreground, background] = config.main_colors;

        // Update CSS colors (background and separator)
        let binding = self.imp().css_provider.borrow();
        let css_provider = binding.as_ref().unwrap();
        let new_css = BASE_CSS.replace("#000000", &rgba_to_hex(&background));
        css_provider.load_from_data(&new_css);

        self.refresh_terminals();
    }

    fn refresh_terminals(&self) {
        let (font_desc, main_colors, palette_colors, scrollback_lines) = self.get_terminal_config();

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

fn rgba_to_hex(rgba: &RGBA) -> String {
    let red = (rgba.red() * 255.).round() as i32;
    let green = (rgba.green() * 255.).round() as i32;
    let blue = (rgba.blue() * 255.).round() as i32;
    format!("#{:02X}{:02X}{:02X}", red, green, blue)
}
