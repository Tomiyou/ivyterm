mod imp;
mod tmux;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{gdk::{Key, ModifierType}, Align, Box, Button, Orientation, PackType, WindowControls, WindowHandle};
use libadwaita::{gio, glib, prelude::*, Application, ApplicationWindow, TabBar, TabView};

use tmux::parse_tmux_layout;

use crate::{
    global_state::show_settings_window, keyboard::keycode_to_arrow_key, next_unique_tab_id, terminal::Terminal, tmux::{Tmux, TmuxCommand, TmuxEvent}, toplevel::TopLevel
};

glib::wrapper! {
    pub struct IvyWindow(ObjectSubclass<imp::IvyWindowPriv>)
        @extends ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
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
        let imp = self.imp();

        // First store Tmux
        imp.tmux.replace(Some(tmux));

        // Then get initial layout - this order to prevent a possible race condition
        let binding = imp.tmux.borrow();
        let tmux = binding.as_ref().unwrap();
        tmux.get_initial_layout();
    }

    pub fn new_tab(&self, id: Option<u32>) -> TopLevel {
        let tab_id = if let Some(id) = id {
            id
        } else {
            next_unique_tab_id()
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

    pub fn is_tmux(&self) -> bool {
        let binding = self.imp().tmux.borrow();
        binding.is_some()
    }

    pub fn register_terminal(&self, pane_id: u32, terminal: &Terminal) {
        let mut terminals = self.imp().terminals.borrow_mut();
        terminals.insert(pane_id, terminal.clone());
        println!("Terminal with ID {} registered", pane_id);
    }

    pub fn unregister_terminal(&self, pane_id: u32) {
        let mut terminals = self.imp().terminals.borrow_mut();
        terminals.remove(&pane_id);
        println!("Terminal with ID {} unregistered", pane_id);
    }

    pub fn tmux_keypress(&self, pane_id: u32, keycode: u32, keyval: Key, state: ModifierType) {
        let binding = self.imp().tmux.borrow();
        let tmux = binding.as_ref().unwrap();

        let mut prefix = String::new();
        let mut shift_relevant = false;
        if state.contains(ModifierType::ALT_MASK) {
            prefix.push_str("M-");
            shift_relevant = true;

            // Hacky workaround for Alt+Backspace
            if keycode == 22 {
                tmux.send_keypress(pane_id, '\x7f', prefix, None);
                return;
            }
        }
        if state.contains(ModifierType::CONTROL_MASK) {
            prefix.push_str("C-");
            shift_relevant = true;
        }
        // Uppercase characters work without S-, so this case is only
        // relevant when Ctrl/Alt is also pressed
        if state.contains(ModifierType::SHIFT_MASK) && shift_relevant {
            prefix.push_str("S-");
        }

        if let Some(c) = keyval.to_unicode() {
            tmux.send_keypress(pane_id, c, prefix, None);
        } else if let Some(direction) = keycode_to_arrow_key(keycode) {
            let direction = match direction {
                crate::keyboard::Direction::Left => "Left",
                crate::keyboard::Direction::Right => "Right",
                crate::keyboard::Direction::Up => "Up",
                crate::keyboard::Direction::Down => "Down",
            };
            tmux.send_keypress(pane_id, ' ', prefix, Some(direction));
        }
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
        let pane = terminals.get(&id);
        if let Some(pane) = pane {
            return Some(pane.clone());
        }

        None
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
        let mut binding = self.imp().tmux.borrow_mut();
        let tmux = binding.as_mut().unwrap();
        tmux.change_size(80, 30);
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

    pub fn tmux_event_callback(&self, event: TmuxEvent) {
        // This future runs on main thread of GTK application
        // It receives Tmux events from separate thread and runs GTK functions
        match event {
            TmuxEvent::LayoutChanged(layout) => {
                println!("Given layout: {}", std::str::from_utf8(&layout).unwrap());
                parse_tmux_layout(&layout[1..], &self);
                // TODO: Move this somewhere better
                self.tmux_resize_window();
                self.tmux_inital_output();
            }
            TmuxEvent::Output(pane_id, output) => {
                let binding = &self.imp().terminals;
                if let Some(pane) = binding.borrow().get(&pane_id) {
                    pane.feed_output(output);
                }
            }
            TmuxEvent::Exit => {
                println!("Received EXIT event, closing window!");
                self.close();
            }
            TmuxEvent::ScrollOutput(pane_id, empty_lines) => {
                let binding = &self.imp().terminals;
                if let Some(pane) = binding.borrow().get(&pane_id) {
                    pane.scroll_view(empty_lines);
                }
            }
        }
    }

    pub fn tmux_layout_callback() {

    }
}
