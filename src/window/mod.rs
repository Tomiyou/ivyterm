mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{gdk::Key, Align, Box, Button, Orientation, PackType, WindowControls, WindowHandle};
use libadwaita::{gio, glib, prelude::*, Application, ApplicationWindow, TabBar, TabView};

use crate::{
    global_state::show_settings_window, next_unique_tab_id, pane::Pane, tmux::{Tmux, TmuxCommand}, toplevel::TopLevel
};

glib::wrapper! {
    pub struct IvyWindow(ObjectSubclass<imp::IvyWindowPriv>)
        @extends ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
        // @extends gtk::Button, gtk::Widget;
}

impl IvyWindow {
    pub fn new(app: &Application, title: &str, default_width: i32, default_height: i32) -> Self {
        let window: Self = Object::builder().build();
        window.set_application(Some(app));
        window.set_title(Some(title));
        window.set_default_width(default_width);
        window.set_default_height(default_height);

        println!("Created new window!");

        // Window content box holds title bar and panes
        let window_box = Box::new(Orientation::Vertical, 0);

        // View stack holds all panes
        let tab_view = TabView::new();
        let mut binding = window.imp().tab_view.borrow_mut();
        binding.replace(tab_view.clone());
        drop(binding);

        // Close the tab_view when 0 tabs remain
        let _window = window.clone();
        tab_view.connect_close_page(move |tab_view, _page| {
            if tab_view.n_pages() < 2 {
                _window.close();
            }
            false
        });

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
        // window.connect_realize();

        window
    }

    pub fn init_tmux(&self, tmux: Tmux) {
        // Get the initial layout
        tmux.send_command(TmuxCommand::InitialLayout);
        self.imp().tmux.replace(Some(tmux));
    }

    pub fn new_tab(&self, id: Option<u32>) -> TopLevel {
        let tab_id = if let Some(id) = id {
            id
        } else {
            next_unique_tab_id()
        };

        let binding = self.imp().tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();

        // Create new TopLevel widget
        let top_level = TopLevel::new(tab_view, self);

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

    pub fn is_tmux(&self) -> bool {
        let binding = self.imp().tmux.borrow();
        binding.is_some()
    }

    pub fn register_terminal(&self, pane_id: u32, terminal: &Pane) {
        let mut terminals = self.imp().terminals.borrow_mut();
        terminals.insert(pane_id, terminal.clone());
        println!("Terminal with ID {} registered", pane_id);
    }

    pub fn unregister_terminal(&self, pane_id: u32) {
        let mut terminals = self.imp().terminals.borrow_mut();
        terminals.remove(&pane_id);
        println!("Terminal with ID {} unregistered", pane_id);
    }

    pub fn tmux_keypress(&self, pane_id: u32, key: u32, keyval: Key) {
        let binding = self.imp().tmux.borrow();
        let tmux = binding.as_ref().unwrap();

        match key {
            // Ignore Ctrl, Shift, Super
            37 | 50 | 133 => {}
            // Handle Enter separately
            36 => {
                tmux.send_keypress(0, '\n');
            }
            // Handle the rest
            c => {
                if let Some(c) = keyval.to_unicode() {
                    tmux.send_keypress(pane_id, c);
                }
            }
        }
    }

    // pub fn get_pane(&self, pane_id: u32) -> Option<&Pane> {
    //     let binding = self.imp().terminals;
    //     let pane = binding.borrow().get(&pane_id);
    // }

    pub fn output_on_pane(&self, pane_id: u32, output: Vec<u8>) {
        let binding = &self.imp().terminals;
        if let Some(pane) = binding.borrow().get(&pane_id) {
            pane.feed_output(output)
            
        }
    }

    // pub fn get_tmux_cols_rows(&self) -> (i32, i32) {
    //     let imp = self.imp();
    //     let binding = imp.tabs.borrow();
    //     let top_level = binding.first().unwrap();
    //     let width = top_level.size(Orientation::Horizontal);
    //     let height = top_level.size(Orientation::Vertical);
    //     let binding = imp.terminals.borrow();
    //     // let terminal = binding.iter
    // }

    pub fn tmux_resize_window(&self) {
        let binding = self.imp().tmux.borrow();
        let tmux = binding.as_ref().unwrap();
        tmux.send_command(TmuxCommand::ChangeSize(80, 24));
    }

    pub fn tmux_inital_output(&self) {
        let imp = self.imp();
        let binding = imp.tmux.borrow();
        let tmux = binding.as_ref().unwrap();

        let terminals = imp.terminals.borrow();
        for (pane_id, _) in terminals.iter() {
            tmux.send_command(TmuxCommand::InitialOutput(*pane_id));
        }
    }
}
