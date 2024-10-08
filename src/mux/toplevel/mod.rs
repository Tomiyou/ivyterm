mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{graphene::Rect, Orientation};
use libadwaita::{glib, prelude::*, TabView};
use vte4::{Terminal, WidgetExt};

use crate::{
    global_state::SPLIT_HANDLE_WIDTH,
    keyboard::Direction,
    mux::{pane::new_paned, terminal::create_terminal},
};

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

    pub fn split(&self, orientation: Orientation) {
        let old_terminal = self.child().unwrap();
        let new_terminal = create_terminal(&self);

        self.set_child(None::<&Self>);
        let new_paned = new_paned(orientation, old_terminal, new_terminal);
        self.set_child(Some(&new_paned));
    }

    pub fn new_terminal(&self, terminal: &Terminal) {
        let mut binding = self.imp().terminals.borrow_mut();
        binding.push(terminal.clone());
    }

    pub fn close_terminal(&self, terminal: &Terminal) {
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
