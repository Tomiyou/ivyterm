mod imp;
mod layout;
mod tmux;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::Widget;
use libadwaita::{glib, prelude::*, TabView};
use log::debug;

use crate::{helpers::borrow_clone, modals::spawn_rename_modal};

use self::imp::Zoomed;

use super::{container::TmuxContainer, terminal::TmuxTerminal, IvyTmuxWindow};

glib::wrapper! {
    pub struct TmuxTopLevel(ObjectSubclass<imp::TopLevelPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl TmuxTopLevel {
    pub fn new(tab_view: &TabView, window: &IvyTmuxWindow, tab_id: u32) -> Self {
        let top_level: TmuxTopLevel = Object::builder().build();
        top_level.set_vexpand(true);
        top_level.set_hexpand(true);
        top_level.set_focusable(true);

        top_level.imp().init_values(tab_view, window, tab_id);

        top_level
    }

    pub fn tab_id(&self) -> u32 {
        self.imp().tab_id.get()
    }

    pub fn zoom(&self, term_id: u32, terminal: TmuxTerminal) -> Zoomed {
        // Remove Terminal from its parent Container
        let container: TmuxContainer = terminal.parent().unwrap().downcast().unwrap();
        let previous_sibling = terminal.prev_sibling();
        terminal.unparent();

        // Remove root Container and replace it with Terminal
        let root_container: TmuxContainer = self.child().unwrap().downcast().unwrap();
        self.set_child(Some(&terminal));
        terminal.grab_focus();

        Zoomed {
            term_id,
            terminal: terminal,
            root_container,
            terminal_container: container,
            previous_sibling,
        }
    }

    pub fn unzoom(&self, z: Zoomed) {
        self.set_child(None::<&Widget>);
        z.terminal
            .insert_after(&z.terminal_container, z.previous_sibling.as_ref());

        self.set_child(Some(&z.root_container));
        z.terminal.grab_focus();
    }

    pub fn register_terminal(&self, terminal: &TmuxTerminal) {
        let pane_id = terminal.id();
        let imp = self.imp();

        let mut terminals_vec = imp.terminals.borrow_mut();
        terminals_vec.push(terminal.clone());

        // Also update global terminal hashmap
        let window = borrow_clone(&imp.window);
        window.register_terminal(pane_id, terminal);
    }

    pub fn gtk_terminal_focus_changed(&self, term_id: u32) {
        let imp = self.imp();

        let old_term = imp.focused_terminal.replace(term_id);
        if old_term != term_id {
            // Focused Terminal changed, we should notify Tmux of this
            let window = borrow_clone(&imp.window);
            window.gtk_terminal_focus_changed(term_id);
        }
    }

    pub fn select_terminal_event(&self, term_id: u32) {
        // TODO: Maybe our implementation of focus tracking is better than Tmux?
        let imp = self.imp();
        imp.focused_terminal.replace(term_id);

        // Grab focus on the correct Terminal
        for terminal in imp.terminals.borrow().iter() {
            if terminal.id() == term_id {
                terminal.grab_focus();
                break;
            }
        }
    }

    pub fn get_cols_rows(&self) -> (i32, i32) {
        let terminals = self.imp().terminals.borrow();
        if let Some(terminal) = terminals.first() {
            let allocation = self.allocation();
            let (char_width, char_height) = terminal.get_char_width_height();
            // VTE widget has a fixed padding of 1px on each side
            let cols = (allocation.width() - 2) / char_width;
            let rows = (allocation.height() - 2) / char_height;
            debug!(
                "Cols: {} | total width {} char width {}",
                cols,
                allocation.width(),
                char_width
            );
            debug!(
                "Rows: {} | total width {} char height {}",
                rows,
                allocation.height(),
                char_height
            );
            return (cols, rows);
        }

        (0, 0)
    }

    pub fn layout_alloc_changed(&self) {
        let window = borrow_clone(&self.imp().window);
        window.resync_tmux_size();
    }

    pub fn adjust_separator_positions(&self, x_diff: f64, y_diff: f64) {
        if x_diff == 1f64 && y_diff == 1f64 {
            return;
        }

        debug!(
            "Temporarily adjusting Separator positions (x: {}, y: {})",
            x_diff, y_diff
        );

        if let Some(child) = self.child() {
            if let Ok(container) = child.downcast::<TmuxContainer>() {
                container.recursive_separator_adjust(x_diff, y_diff);
            }
        }
    }

    pub fn focus_current_terminal(&self) {
        let imp = self.imp();

        // TODO: Fix this, Tmux currently does not report active Pane
        // Ensure the correct Pane is focused
        let focused_terminal = imp.focused_terminal.get();
        let registered_terminals = imp.terminals.borrow();
        for terminal in registered_terminals.iter() {
            if terminal.id() == focused_terminal {
                terminal.grab_focus();
                break;
            }
        }
    }

    pub fn open_rename_modal(&self) {
        let imp = self.imp();
        let window = borrow_clone(&imp.window);
        let tab_id = imp.tab_id.get();

        let callback = glib::closure_local!(
            #[weak]
            window,
            move |new_name: &str| {
                window.rename_tmux_tab(tab_id, new_name);
            }
        );

        // We need the "parent" Window for modal
        let parent = borrow_clone(&imp.window);
        spawn_rename_modal(parent.upcast_ref(), "", callback);
    }

    pub fn tab_renamed(&self, new_name: &str) {
        let imp = self.imp();
        let tab_view = borrow_clone(&imp.tab_view);

        // TODO: Just store the Page directly instead of tab_view
        let page = tab_view.page(self);
        page.set_title(new_name);
    }
}
