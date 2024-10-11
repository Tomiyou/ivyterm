mod imp;

use std::sync::atomic::Ordering;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Propagation};
use gtk4::{
    gdk::RGBA, pango::FontDescription, Align, Box, Button, CssProvider, Orientation, PackType, ShortcutController, WindowControls, WindowHandle
};
use libadwaita::{gio, glib, prelude::*, TabBar, TabView};
use log::debug;

use crate::{
    application::IvyApplication, modals::spawn_new_tmux_modal, settings::{APPLICATION_TITLE, INITIAL_HEIGHT, INITIAL_WIDTH}
};

use super::{terminal::Terminal, toplevel::TopLevel};

glib::wrapper! {
    pub struct IvyNormalWindow(ObjectSubclass<imp::IvyWindowPriv>)
        @extends libadwaita::ApplicationWindow, gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl IvyNormalWindow {
    pub fn new(app: &IvyApplication, css_provider: &CssProvider) -> Self {
        let window: Self = Object::builder().build();
        window.set_application(Some(app));
        window.set_title(Some(APPLICATION_TITLE));
        window.set_default_width(INITIAL_WIDTH);
        window.set_default_height(INITIAL_HEIGHT);

        // Window content box holds title bar and panes
        let window_box = Box::new(Orientation::Vertical, 0);

        // View stack holds all panes
        let tab_view = TabView::new();
        // Remove default Shortcut Controller
        {
            let controllers = tab_view.observe_controllers();
            let mut i = 0;
            while let Some(ctrl) = controllers.item(i) {
                if let Ok(ctrl) = ctrl.downcast::<ShortcutController>() {
                    println!("Removing Shortcut controller!");
                    tab_view.remove_controller(&ctrl);
                } else {
                    i += 1;
                }
            }
        }
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
        let tmux_button = Button::with_label("Tmux");
        tmux_button.connect_clicked(glib::clone!(
            #[strong]
            window,
            move |_| {
                spawn_new_tmux_modal(window.upcast_ref());
            }
        ));
        // Tmux session spawn
        let settings_button = Button::with_label("Settings");
        settings_button.connect_clicked(glib::clone!(
            #[strong]
            app,
            move |_| {
                app.show_settings();
            }
        ));
        // HeaderBar end widgets
        let end_widgets = Box::new(Orientation::Horizontal, 3);
        end_widgets.append(&tmux_button);
        end_widgets.append(&settings_button);

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
            .end_action_widget(&end_widgets)
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

        // Spawn the first tab
        window.new_tab();

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

    pub fn new_tab(&self) -> TopLevel {
        let imp = self.imp();
        let tab_id = self.unique_tab_id();

        let binding = imp.tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();

        // Create new TopLevel widget
        let top_level = TopLevel::new(tab_view, self, tab_id);
        let mut tabs = imp.tabs.borrow_mut();
        tabs.push(top_level.clone());

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
        let imp = self.imp();
        let mut terminals = imp.terminals.borrow_mut();
        terminals.insert(pane_id, &terminal);
        debug!("Terminal with ID {} registered", pane_id);
    }

    pub fn unregister_terminal(&self, pane_id: u32) {
        let mut terminals = self.imp().terminals.borrow_mut();
        terminals.remove(pane_id);
        debug!("Terminal with ID {} unregistered", pane_id);
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
            sorted
                .terminal
                .update_config(font_desc, main_colors, palette_colors, scrollback_lines);
        }
    }
}
