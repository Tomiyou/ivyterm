mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{graphene::Rect, Orientation, Paned, Widget};
use libadwaita::{glib, prelude::*, TabView};
use vte4::{Terminal, WidgetExt};

use crate::{
    global_state::SPLIT_HANDLE_WIDTH, keyboard::Direction, mux::new_paned, terminal::create_terminal
};

use self::imp::Zoomed;

use super::create_tab;

glib::wrapper! {
    pub struct TopLevel(ObjectSubclass<imp::TopLevel>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl TopLevel {
    pub fn new(tab_view: &TabView) -> Self {
        let top_level: TopLevel = Object::builder().build();

        top_level
            .imp()
            .tab_view
            .borrow_mut()
            .replace(tab_view.clone());

        let terminal = create_terminal(&top_level);

        top_level.set_vexpand(true);
        top_level.set_hexpand(true);
        top_level.set_focusable(true);
        top_level.set_child(Some(&terminal));

        top_level
    }

    pub fn create_tab(&self) {
        let binding = self.imp().tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();
        create_tab(tab_view);
    }

    pub fn close_tab(&self) {
        let binding = self.imp().tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();
        let page = tab_view.page(self);
        tab_view.close_page(&page);
    }

    pub fn split_pane(&self, terminal: &Terminal, orientation: Orientation) {
        self.unzoom();

        let new_terminal = create_terminal(&self);

        let parent = terminal.parent().unwrap();
        if parent.eq(self) {
            // Terminal's parent is myself
            let old_terminal = self.child().unwrap();

            self.set_child(None::<&Self>);
            let new_paned = new_paned(orientation, old_terminal, new_terminal);
            self.set_child(Some(&new_paned));
            return;
        }
        // Terminal's parent is a Paned widget

        let paned: Paned = parent.downcast().unwrap();
        let start_child = paned.start_child().unwrap();
        if start_child.eq(terminal) {
            // Replace first child
            paned.set_start_child(None::<&Widget>);

            let new_paned = new_paned(orientation, start_child, new_terminal);
            paned.set_start_child(Some(&new_paned));

            // Re-calculate panes divider position
            let size = paned.size(paned.orientation());
            paned.set_position(size / 2);
            return;
        }

        let end_child = paned.end_child().unwrap();
        if end_child.eq(terminal) {
            // Replace end child
            paned.set_end_child(None::<&Widget>);

            let new_paned = new_paned(orientation, end_child, new_terminal);
            paned.set_end_child(Some(&new_paned));

            // Re-calculate panes divider position
            let size = paned.size(paned.orientation());
            paned.set_position(size / 2);
            return;
        }
    }

    pub fn close_pane(&self, closing_terminal: &Terminal) {
        self.unzoom();

        let parent = closing_terminal.parent().unwrap();
        if parent.eq(self) {
            // Parent of the closing terminal is myself, we need to close this tab
            self.close_tab();
            return;
        }

        // Parent of the closing terminal is a Paned widget
        let closing_paned: Paned = parent.downcast().unwrap();

        // Paned always has 2 children present, if not, then it would have been deleted
        let start_child = closing_paned.start_child().unwrap();
        let end_child = closing_paned.end_child().unwrap();

        let (retained_child, direction) = if start_child.eq(closing_terminal) {
            // Remove start child, keep last child
            let direction = match closing_paned.orientation() {
                Orientation::Horizontal => Direction::Right,
                Orientation::Vertical => Direction::Down,
                _ => panic!("Orientation not horizontal or vertical"),
            };
            (end_child, direction)
        } else if end_child.eq(closing_terminal) {
            // Remove last child, keep first child
            let direction = match closing_paned.orientation() {
                Orientation::Horizontal => Direction::Left,
                Orientation::Vertical => Direction::Up,
                _ => panic!("Orientation not horizontal or vertical"),
            };
            (start_child, direction)
        } else {
            panic!("Trying to close pane, but none of the children is the closed terminal");
        };

        // Find terminal to focus after the closing terminal is unrealized
        let new_focus = self
            .find_neighbor(&closing_terminal, direction)
            .or_else(|| Some(retained_child.clone().downcast::<Terminal>().unwrap()))
            .unwrap();
        new_focus.grab_focus();

        closing_paned.set_start_child(None::<&Widget>);
        closing_paned.set_end_child(None::<&Widget>);

        // Determine if parent is type Bin or Paned
        let parent = closing_paned.parent().unwrap();

        if let Ok(parent) = parent.clone().downcast::<TopLevel>() {
            // Parent is TopLevel
            parent.set_child(Some(&retained_child));
            new_focus.grab_focus();
            return;
        }

        if let Ok(parent) = parent.downcast::<Paned>() {
            // Parent is another gtk4::Paned
            parent.emit_cycle_child_focus(true);

            let start_child = parent.start_child().unwrap();
            let end_child = parent.end_child().unwrap();

            if start_child == closing_paned {
                // Closing Pane is start child
                parent.set_start_child(Some(&retained_child));
            } else if end_child == closing_paned {
                // Closing Pane is end child
                parent.set_end_child(Some(&retained_child));
            } else {
                panic!("Closing Pane is neither first nor end child");
            }

            // Re-adjust split positions
            let size = parent.size(parent.orientation());
            parent.set_position(size / 2);

            // Grab focus for a new terminal
            new_focus.grab_focus();

            return;
        }

        panic!("Parent is neither Bin nor Paned");
    }

    pub fn toggle_zoom(&self, terminal: &Terminal) {
        let imp = self.imp();
        let binding = imp.terminals.borrow();
        if binding.len() < 2 {
            // There is only 1 terminal present, no need for any zooming
            return;
        }

        let mut binding = imp.zoomed.borrow_mut();
        if let Some(zoomed) = binding.take() {
            // Unzoom the terminal
            self.set_child(None::<&Widget>);
            if zoomed.is_start_child {
                zoomed.terminal_paned.set_start_child(Some(&zoomed.terminal));
            } else {
                zoomed.terminal_paned.set_end_child(Some(&zoomed.terminal));
            }

            self.set_child(Some(&zoomed.root_paned));
            terminal.grab_focus();
            return;
        }
        // Zoom the terminal

        // Remove Terminal from its parent Paned
        let terminal_paned: Paned = terminal.parent().unwrap().downcast().unwrap();
        let is_start_child = if terminal_paned.start_child().unwrap().eq(terminal) {
            terminal_paned.set_start_child(None::<&Widget>);
            true
        } else {
            terminal_paned.set_end_child(None::<&Widget>);
            false
        };

        // Remove root Paned and replace it with Terminal
        let root_paned: Paned = self.child().unwrap().downcast().unwrap();
        self.set_child(Some(terminal));
        terminal.grab_focus();

        let zoomed = Zoomed {
            terminal: terminal.clone(),
            root_paned,
            terminal_paned,
            is_start_child
        };
        binding.replace(zoomed);
    }

    pub fn unzoom(&self) {
        let imp = self.imp();
        let binding = imp.terminals.borrow();
        if binding.len() < 2 {
            // There is only 1 terminal present, no need for any unzooming
            return;
        }

        let mut binding = imp.zoomed.borrow_mut();
        if let Some(zoomed) = binding.take() {
            self.set_child(None::<&Widget>);
            if zoomed.is_start_child {
                zoomed.terminal_paned.set_start_child(Some(&zoomed.terminal));
            } else {
                zoomed.terminal_paned.set_end_child(Some(&zoomed.terminal));
            }

            self.set_child(Some(&zoomed.root_paned));
            zoomed.terminal.grab_focus();
        }
    }

    pub fn register_terminal(&self, terminal: &Terminal) {
        let mut binding = self.imp().terminals.borrow_mut();
        binding.push(terminal.clone());
    }

    pub fn unregister_terminal(&self, terminal: &Terminal) {
        let mut binding = self.imp().terminals.borrow_mut();
        binding.retain(|t| t != terminal);
    }

    pub fn find_neighbor(&self, terminal: &Terminal, direction: Direction) -> Option<Terminal> {
        let binding = self.imp().terminals.borrow();
        if binding.len() < 2 {
            return None;
        }

        const PAD: f32 = SPLIT_HANDLE_WIDTH as f32 + 5.0;

        // We will use Rect intersection to find a matching neighbor. For this to work, the Rect
        // used for calculating the intersection must be slightly larger in the direction we
        // wish to find a neighbor.
        let (_, _, width, height) = terminal.bounds().unwrap();
        let terminal_rect = match direction {
            Direction::Up => Rect::new(0.0, -PAD, width as f32, height as f32 + PAD),
            Direction::Down => Rect::new(0.0, 0.0, width as f32, height as f32 + PAD),
            Direction::Left => Rect::new(-PAD, 0.0, width as f32 + PAD, height as f32),
            Direction::Right => Rect::new(0.0, 0.0, width as f32 + PAD, height as f32),
        };

        // Loop through all the terminals in the window and find a suitable neighbor
        for neighbor in binding.iter() {
            if neighbor == terminal {
                continue;
            }

            // terminal.compute_bounds(&target_terminal) calculates the distance between terminals
            // and returns a Rect graphene struct which contains x and y distance from the target
            // terminal, and width and height of the neighbor
            let bounds = neighbor.compute_bounds(terminal).unwrap();
            let intersection = terminal_rect.intersection(&bounds);
            if intersection.is_some() {
                return Some(neighbor.clone());
            }
        }

        None
    }
}
