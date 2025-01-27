mod imp;
mod tmux;

use std::rc::Rc;

use glib::{subclass::types::ObjectSubclassIsExt, Object, Propagation};
use gtk4::{Align, Box, Button, Orientation, PackType, WindowControls, WindowHandle};
use libadwaita::{gio, glib, prelude::*, ApplicationWindow, TabBar, TabView};
use log::debug;
use tmux::TmuxInitState;

use crate::{
    application::IvyApplication,
    config::{TerminalConfig, APPLICATION_TITLE, INITIAL_HEIGHT, INITIAL_WIDTH},
    helpers::borrow_clone,
    keyboard::KeyboardAction,
    modals::spawn_new_tmux_modal,
    ssh::{new_session, SSHData},
    tmux_api::TmuxAPI,
};

use super::{terminal::TmuxTerminal, toplevel::TmuxTopLevel};

#[macro_export]
macro_rules! close_on_error {
    ( $e:expr, $window:ident ) => {
        if let Err(_) = $e {
            $window.close();
            return;
        }
    };
}

#[inline]
fn get_tmux_ref(window: &IvyTmuxWindow) -> Option<Rc<TmuxAPI>> {
    window.imp().tmux.borrow().clone()
}

glib::wrapper! {
    pub struct IvyTmuxWindow(ObjectSubclass<imp::IvyWindowPriv>)
        @extends ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl IvyTmuxWindow {
    pub fn new(app: &IvyApplication, tmux_session: &str, ssh_host: Option<(&str, &str)>) -> Self {
        let window: Self = Object::builder().build();
        window.set_application(Some(app));
        window.set_title(Some(APPLICATION_TITLE));
        window.set_default_width(INITIAL_WIDTH);
        window.set_default_height(INITIAL_HEIGHT);
        window.add_css_class("tmux_window");

        // Window content box holds title bar and panes
        let window_box = Box::new(Orientation::Vertical, 0);

        // View stack holds all panes
        let tab_view = TabView::new();
        window.imp().initialize(&tab_view);

        // Close Window automatically, when all pages (Tabs) have been closed
        tab_view.connect_n_pages_notify(glib::clone!(
            #[weak]
            window,
            move |tab_view| {
                if tab_view.n_pages() < 1 {
                    window.close();
                }
            }
        ));
        // Automatically remove unregister Tabs/Terminals when their respective
        // page is closed
        tab_view.connect_close_page(glib::clone!(
            #[weak]
            window,
            #[upgrade_or]
            Propagation::Proceed,
            move |_, closing_page| {
                // Unregister all Terminals owned by this closing tab
                let imp = window.imp();
                let closing_tab: TmuxTopLevel = closing_page.child().downcast().unwrap();

                let closed_terminals = closing_tab.imp().terminals.borrow().clone();
                imp.terminals.borrow_mut().retain(|terminal| {
                    for closed in closed_terminals.iter() {
                        if terminal.terminal.eq(closed) {
                            debug!("Unregistered Terminal {} since Tab was closed", terminal.id);
                            return false;
                        }
                    }

                    true
                });

                // Remove the tab from the tab list
                imp.tabs.borrow_mut().retain(|tab| !closing_tab.eq(tab));

                // This is a hacky fix of what appears to be a libadwaita issue.
                // The issue is reproducible in 1.5.0 and resolved in 1.6.0. Not
                // sure if 1.5.x versions have been fixed.
                if libadwaita::major_version() < 2 && libadwaita::minor_version() < 6 {
                    closing_page.child().unparent();
                }

                Propagation::Proceed
            }
        ));

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

        if let Some((ssh_target, ssh_password)) = ssh_host {
            new_ssh_session(&window, tmux_session, ssh_target, ssh_password);
        } else {
            window.initialize_tmux(tmux_session, None);
        }

        window
    }

    /// Called after both Tmux and SSH session are ready (if it exists)
    fn initialize_tmux(&self, tmux_session: &str, ssh_data: Option<SSHData>) {
        // Initialize Tmux API
        let tmux = TmuxAPI::new(tmux_session, ssh_data, self).unwrap();
        self.imp().tmux.replace(Some(Rc::new(tmux)));

        // Get initial Tmux layout
        if let Some(tmux) = get_tmux_ref(self) {
            close_on_error!(tmux.get_initial_layout(), self);
        }
    }

    pub fn new_tab(&self, id: u32) -> TmuxTopLevel {
        let imp = self.imp();
        let tab_view = borrow_clone(&imp.tab_view);

        // Create new TopLevel widget
        let top_level = TmuxTopLevel::new(&tab_view, self, id);
        let mut tabs = imp.tabs.borrow_mut();
        tabs.push(top_level.clone());

        // Add pane as a page
        // TODO: Do this once for tab_view instead of each page
        let page = tab_view.append(&top_level);
        page.connect_selected_notify(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |page| {
                if page.is_selected() {
                    window.gtk_tab_focus_changed(id);
                }
            }
        ));

        let text = format!("Terminal {}", id);
        page.set_title(&text);
        tab_view.set_selected_page(&page);

        top_level
    }

    pub fn close_tab(&self, closing_tab: &TmuxTopLevel) {
        let imp = self.imp();
        let tab_view = borrow_clone(&imp.tab_view);
        let page = tab_view.page(closing_tab);
        tab_view.close_page(&page);
    }

    pub fn register_terminal(&self, pane_id: u32, terminal: &TmuxTerminal) {
        let imp = self.imp();
        let mut terminals = imp.terminals.borrow_mut();
        terminals.insert(pane_id, &terminal);
        debug!("Terminal with ID {} registered", pane_id);

        let char_size = terminal.get_char_width_height();
        imp.char_size.replace(char_size);
    }

    pub fn unregister_terminal(&self, pane_id: u32) {
        let mut terminals = self.imp().terminals.borrow_mut();
        terminals.remove(pane_id);
        debug!("Terminal with ID {} unregistered", pane_id);
    }

    fn get_top_level(&self, id: u32) -> Option<TmuxTopLevel> {
        let tabs = self.imp().tabs.borrow();
        for top_level in tabs.iter() {
            if top_level.tab_id() == id {
                return Some(top_level.clone());
            }
        }

        None
    }

    pub fn get_terminal_by_id(&self, id: u32) -> Option<TmuxTerminal> {
        let terminals = self.imp().terminals.borrow();
        let pane = terminals.get(id);
        if let Some(pane) = pane {
            return Some(pane.clone());
        }

        None
    }

    pub fn update_terminal_config(&self, config: &TerminalConfig) {
        let terminals = self.imp().terminals.borrow();
        for sorted in terminals.iter() {
            sorted.terminal.update_config(config);
        }
    }

    #[inline]
    pub fn tmux_handle_keybinding(&self, action: KeyboardAction, pane_id: u32) {
        if let Some(tmux) = get_tmux_ref(self) {
            close_on_error!(tmux.send_keybinding(action, pane_id), self);
        }
    }

    pub fn gtk_terminal_focus_changed(&self, term_id: u32) {
        if let Some(tmux) = get_tmux_ref(self) {
            close_on_error!(tmux.select_terminal(term_id), self);
        }
    }

    pub fn gtk_tab_focus_changed(&self, tab_id: u32) {
        let imp = self.imp();

        if imp.init_layout_finished.get() == TmuxInitState::Done {
            imp.focused_tab.replace(tab_id);

            if let Some(tmux) = get_tmux_ref(self) {
                close_on_error!(tmux.select_tab(tab_id), self);
            }
        }
    }

    pub fn clipboard_paste_event(&self, pane_id: u32) {
        let clipboard = self.primary_clipboard();
        let future = clipboard.read_text_future();

        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = window)]
            self,
            async move {
                if let Ok(output) = future.await {
                    if let Some(output) = output {
                        window.send_clipboard(pane_id, output.as_str());
                    }
                }
            }
        ));
    }
}

fn new_ssh_session(
    window: &IvyTmuxWindow,
    tmux_session: &str,
    ssh_target: &str,
    ssh_password: &str,
) {
    let tmux_session = tmux_session.to_string();
    let ssh_target = ssh_target.to_string();
    let ssh_password = ssh_password.to_string();

    glib::spawn_future_local(glib::clone!(
        #[weak]
        window,
        async move {
            let ret =
                match gio::spawn_blocking(move || new_session(&ssh_target, &ssh_password)).await {
                    Ok(ret) => ret,
                    Err(_) => {
                        window.close();
                        return;
                    }
                };

            let tuple = match ret {
                Ok(ret) => ret,
                Err(_) => {
                    window.close();
                    return;
                }
            };

            window.initialize_tmux(&tmux_session, Some(tuple));
        }
    ));
}
