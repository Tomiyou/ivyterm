mod imp;

use std::sync::atomic::Ordering;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{Align, Box, Button, Orientation, PackType, WindowControls, WindowHandle};
use libadwaita::{gio, glib, prelude::*, TabBar, TabView};
use log::debug;

use crate::{
    application::IvyApplication,
    config::{TerminalConfig, APPLICATION_TITLE, INITIAL_HEIGHT, INITIAL_WIDTH},
    modals::spawn_new_tmux_modal,
};

use super::{terminal::Terminal, toplevel::TopLevel};

glib::wrapper! {
    pub struct IvyNormalWindow(ObjectSubclass<imp::IvyWindowPriv>)
        @extends libadwaita::ApplicationWindow, gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl IvyNormalWindow {
    pub fn new(app: &IvyApplication) -> Self {
        let window: Self = Object::builder().build();
        window.set_application(Some(app));
        window.set_title(Some(APPLICATION_TITLE));
        window.set_default_width(INITIAL_WIDTH);
        window.set_default_height(INITIAL_HEIGHT);

        // Window content box holds title bar and panes
        let window_box = Box::new(Orientation::Vertical, 0);

        // View stack holds all panes
        let tab_view = TabView::new();
        window.imp().initialize(&tab_view);

        // Terminal settings
        let tmux_button = Button::with_label("Tmux");
        tmux_button.connect_clicked(glib::clone!(
            #[weak]
            window,
            move |_| {
                spawn_new_tmux_modal(window.upcast_ref());
            }
        ));
        // Tmux session spawn
        let settings_button = Button::with_label("Settings");
        settings_button.connect_clicked(glib::clone!(
            #[weak]
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
        self.imp().next_tab_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn unique_terminal_id(&self) -> u32 {
        self.imp().next_terminal_id.fetch_add(1, Ordering::Relaxed)
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
        tab_view.set_selected_page(&page);

        top_level
    }

    pub fn close_tab(&self, child: &TopLevel) {
        let imp = self.imp();
        let binding = imp.tab_view.borrow();

        // Close the tab (page) in TabView
        let tab_view = binding.as_ref().unwrap();
        let page = tab_view.page(child);
        tab_view.close_page(&page);

        // Remove the tab from the tab list
        let mut tabs = imp.tabs.borrow_mut();
        tabs.retain(|tab| tab != child);
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

    // TODO: Make this an event
    pub fn tab_closed(&self, deleted_tab: u32, deleted_terms: Vec<u32>) {
        let close_window = {
            // Remove all Terminals belonging to the closed Tab
            let mut terminals = self.imp().terminals.borrow_mut();
            terminals.retain(|term_id| {
                let term_id = term_id.id;
                // If the given term_id is one of the deleted_ids, do NOT retain it
                for deleted_id in deleted_terms.iter() {
                    if term_id == *deleted_id {
                        debug!("Terminal with ID {} unregistered", deleted_id);
                        return false;
                    }
                }

                return true;
            });

            // Just in case (mainly for when users uses CloseTab shortcut)
            let mut tabs = self.imp().tabs.borrow_mut();
            tabs.retain(|tab_id| tab_id.tab_id() != deleted_tab);

            tabs.len() == 0
        };

        if close_window {
            self.close();
        }
    }

    pub fn update_terminal_config(&self, config: &TerminalConfig) {
        let binding = self.imp().terminals.borrow();
        for sorted in binding.iter() {
            sorted.terminal.update_config(config);
        }
    }
}
