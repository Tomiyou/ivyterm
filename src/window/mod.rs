mod imp;
mod layout;
mod tmux;

use std::sync::atomic::Ordering;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Propagation};
use gtk4::{
    gdk::RGBA, pango::FontDescription, Align, Box, Button, CssProvider, Orientation, PackType,
    WindowControls, WindowHandle,
};
use libadwaita::{gio, glib, prelude::*, ApplicationWindow, TabBar, TabView};

use crate::{
    application::IvyApplication,
    settings::{APPLICATION_TITLE, INITIAL_HEIGHT, INITIAL_WIDTH},
    terminal::Terminal,
    toplevel::TopLevel,
};

glib::wrapper! {
    pub struct IvyWindow(ObjectSubclass<imp::IvyWindowPriv>)
        @extends ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl IvyWindow {
    pub fn new(app: &IvyApplication, css_provider: &CssProvider) -> Self {
        let window: Self = Object::builder().build();
        window.set_application(Some(app));
        window.set_title(Some(APPLICATION_TITLE));
        window.set_default_width(INITIAL_WIDTH);
        window.set_default_height(INITIAL_HEIGHT);

        println!("Created new window!");

        // Window content box holds title bar and panes
        let window_box = Box::new(Orientation::Vertical, 0);

        // View stack holds all panes
        let tab_view = TabView::new();
        window.imp().initialize(&tab_view, css_provider);

        // Close the tab_view when 0 tabs remain
        let _window = window.clone();
        tab_view.connect_close_page(move |tab_view, _page| {
            if tab_view.n_pages() < 2 {
                _window.close();
            }
            Propagation::Proceed
        });

        // Terminal settings
        let settings_button = Button::with_label("Settings");
        let app = app.clone();
        settings_button.connect_clicked(move |_button| {
            app.show_settings();
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
        window.set_content(Some(&window_box));

        window
    }

    fn unique_tab_id(&self) -> u32 {
        let binding = self.imp().next_tab_id.borrow();
        binding.fetch_add(1, Ordering::Relaxed)
    }

    pub fn unique_terminal_id(&self) -> u32 {
        let binding = self.imp().next_terminal_id.borrow();
        binding.fetch_add(1, Ordering::Relaxed)
    }

    pub fn new_tab(&self, id: Option<u32>) -> TopLevel {
        let tab_id = if let Some(id) = id {
            id
        } else {
            self.unique_tab_id()
        };

        let is_tmux = self.imp().tmux.borrow().is_some();

        let binding = self.imp().tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();

        // Create new TopLevel widget
        let top_level = TopLevel::new(tab_view, self, tab_id, !is_tmux);

        // Add pane as a page
        let page = tab_view.append(&top_level);

        let text = format!("Terminal {}", tab_id);
        page.set_title(&text);
        tab_view.set_selected_page(&page);

        top_level
    }

    pub fn close_tab(&self, child: &TopLevel) {
        let binding = self.imp().tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();
        let page = tab_view.page(child);
        tab_view.close_page(&page);
    }

    pub fn register_terminal(&self, pane_id: u32, terminal: &Terminal) {
        let mut terminals = self.imp().terminals.borrow_mut();
        terminals.insert(pane_id, &terminal);
        println!("Terminal with ID {} registered", pane_id);
    }

    pub fn unregister_terminal(&self, pane_id: u32) {
        let mut terminals = self.imp().terminals.borrow_mut();
        terminals.remove(pane_id);
        println!("Terminal with ID {} unregistered", pane_id);
    }

    pub fn get_top_level(&self, id: u32) -> Option<TopLevel> {
        let tabs = self.imp().tabs.borrow();
        for top_level in tabs.iter() {
            if top_level.tab_id() == id {
                return Some(top_level.clone());
            }
        }

        None
    }

    pub fn get_pane(&self, id: u32) -> Option<Terminal> {
        let terminals = self.imp().terminals.borrow();
        let pane = terminals.get(id);
        if let Some(pane) = pane {
            return Some(pane.clone());
        }

        None
    }

    pub fn update_terminal_config(
        &self,
        font_desc: &FontDescription,
        main_colors: [RGBA; 2],
        palette_colors: [RGBA; 16],
        scrollback_lines: u32,
    ) {
        let binding = self.imp().terminals.borrow();
        for sorted in binding.iter() {
            sorted.terminal.update_config(font_desc, main_colors, palette_colors, scrollback_lines);
        }
    }
}
