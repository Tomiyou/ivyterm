mod imp;

use std::sync::atomic::Ordering;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{graphene::Rect, Box as Container, Orientation, Widget};
use libadwaita::{glib, prelude::*, TabView};

use crate::{
    global_state::SPLIT_HANDLE_WIDTH, keyboard::Direction, pane::Pane, separator::new_separator,
    GLOBAL_TAB_ID,
};

use self::imp::Zoomed;

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

        let terminal = Pane::new(&top_level);

        top_level.set_vexpand(true);
        top_level.set_hexpand(true);
        top_level.set_focusable(true);
        top_level.set_child(Some(&terminal));

        top_level
    }

    pub fn create_tab(&self, id: Option<u32>) {
        let binding = self.imp().tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();
        create_tab(tab_view, id);
    }

    pub fn close_tab(&self) {
        let binding = self.imp().tab_view.borrow();
        let tab_view = binding.as_ref().unwrap();
        let page = tab_view.page(self);
        tab_view.close_page(&page);
    }

    pub fn split_pane(&self, terminal: &Pane, orientation: Orientation) {
        self.unzoom();

        let new_terminal = Pane::new(&self);
        let new_separator = new_separator(orientation);

        let parent = terminal.parent().unwrap();
        if parent.eq(self) {
            // Terminal's parent is myself
            let old_terminal = self.child().unwrap();

            self.set_child(None::<&Self>);
            let container = Container::new(orientation, 0);
            container.append(&old_terminal);
            container.append(&new_separator);
            container.append(&new_terminal);
            self.set_child(Some(&container));
            return;
        }

        // Terminal's parent is a Container widget
        let container: Container = parent.downcast().unwrap();

        // If the split orientation is the same as Container's orientation, we can simply insert a new Pane
        if container.orientation() == orientation {
            container.insert_child_after(&new_separator, Some(terminal));
            container.insert_child_after(&new_terminal, Some(&new_separator));
            return;
        }

        // The split orientation is different from Container's, meaning we have to insert a new Container
        let new_container = Container::new(orientation, 0);
        container.insert_child_after(&new_container, Some(terminal));
        terminal.unparent();
        new_container.append(terminal);
        new_container.append(&new_separator);
        new_container.append(&new_terminal);
    }

    pub fn close_pane(&self, closing_terminal: &Pane) {
        self.unzoom();

        let parent = closing_terminal.parent().unwrap();
        if parent.eq(self) {
            // Parent of the closing terminal is myself, we need to close this tab
            self.close_tab();
            return;
        }

        // Parent of the closing terminal is a Container widget
        let container: Container = parent.downcast().unwrap();

        // Check if there is a next sibling (Separator) and remove it, otherwise remove previous sibling
        if let Some(separator) = closing_terminal.next_sibling() {
            separator.unparent();
        } else if let Some(separator) = closing_terminal.prev_sibling() {
            separator.unparent();
        } else {
            panic!("Closing terminal has no next/previous sibling!");
        }
        closing_terminal.unparent();

        // If container only has 1 child left, we need to remove it and leave the 1 child in its place
        let retained_child = container.first_child().unwrap();
        if retained_child.next_sibling().is_some() {
            return;
        }

        // Remove the last child from Container, which will be deleted
        retained_child.unparent();

        // Determine if parent is type Bin or Container
        let parent = container.parent().unwrap();

        if let Ok(parent) = parent.clone().downcast::<TopLevel>() {
            // Parent is TopLevel
            parent.set_child(Some(&retained_child));
            return;
        }

        if let Ok(parent) = parent.downcast::<Container>() {
            // Parent is another Container
            parent.insert_child_after(&retained_child, Some(&container));
            container.unparent();
            return;
        }

        panic!("Parent is neither Bin nor Paned");
    }

    pub fn toggle_zoom(&self, terminal: &Pane) {
        let imp = self.imp();
        let binding = imp.terminals.borrow();
        if binding.len() < 2 {
            // There is only 1 terminal present, no need for any zooming
            return;
        }

        let mut binding = imp.zoomed.borrow_mut();
        if let Some(z) = binding.take() {
            // Unzoom the terminal
            self.set_child(None::<&Widget>);
            z.terminal_container
                .insert_child_after(&z.terminal, z.previous_sibling.as_ref());

            self.set_child(Some(&z.root_container));
            z.terminal.grab_focus();
            return;
        }
        // Zoom the terminal

        // We need to remember the current width and height for the unzoom portion
        let (x, y, width, height) = terminal.bounds().unwrap();

        // Remove Terminal from its parent Container
        let terminal_paned: Container = terminal.parent().unwrap().downcast().unwrap();
        let previous_sibling = terminal.prev_sibling();
        terminal.unparent();

        // Remove root Container and replace it with Terminal
        let root_paned: Container = self.child().unwrap().downcast().unwrap();
        self.set_child(Some(terminal));
        terminal.grab_focus();

        let zoomed = Zoomed {
            terminal: terminal.clone(),
            root_container: root_paned,
            terminal_container: terminal_paned,
            previous_sibling,
            previous_bounds: (x, y, width, height),
        };
        binding.replace(zoomed);
    }

    pub fn unzoom(&self) -> Option<(i32, i32, i32, i32)> {
        let mut binding = self.imp().zoomed.borrow_mut();
        if let Some(z) = binding.take() {
            self.set_child(None::<&Widget>);
            z.terminal_container
                .insert_child_after(&z.terminal, z.previous_sibling.as_ref());

            self.set_child(Some(&z.root_container));
            z.terminal.grab_focus();

            return Some(z.previous_bounds);
        }

        None
    }

    pub fn register_terminal(&self, terminal: &Pane) {
        let mut binding = self.imp().terminals.borrow_mut();
        binding.push(terminal.clone());
    }

    pub fn unregister_terminal(&self, terminal: &Pane) {
        let mut binding = self.imp().terminals.borrow_mut();
        binding.retain(|t| t != terminal);
    }

    pub fn find_neighbor(
        &self,
        terminal: &Pane,
        direction: Direction,
        use_size: Option<(i32, i32, i32, i32)>,
    ) -> Option<Pane> {
        let binding = self.imp().terminals.borrow();
        if binding.len() < 2 {
            return None;
        }

        const PAD: f32 = SPLIT_HANDLE_WIDTH as f32 + 5.0;

        // We will use Rect intersection to find a matching neighbor. For this to work, the Rect
        // used for calculating the intersection must be slightly larger in the direction we
        // wish to find a neighbor.
        let (x, y, width, height) = if let Some((x, y, width, height)) = use_size {
            (x as f32, y as f32, width as f32, height as f32)
        } else {
            let (_, _, width, height) = terminal.bounds().unwrap();
            (0.0, 0.0, width as f32, height as f32)
        };
        let terminal_rect = match direction {
            Direction::Up => Rect::new(0.0, -PAD, width, height + PAD),
            Direction::Down => Rect::new(0.0, 0.0, width, height + PAD),
            Direction::Left => Rect::new(-PAD, 0.0, width + PAD, height),
            Direction::Right => Rect::new(0.0, 0.0, width + PAD, height),
        };

        // TODO: it can be NULL when widget is being unzoomed
        // println!("Terminal rect {:?}", terminal_rect);
        // Loop through all the terminals in the window and find a suitable neighbor
        for neighbor in binding.iter() {
            if neighbor == terminal {
                continue;
            }

            // terminal.compute_bounds(&target_terminal) calculates the distance between terminals
            // and returns a Rect graphene struct which contains x and y distance from the target
            // terminal, and width and height of the neighbor
            let mut bounds = neighbor.compute_bounds(terminal).unwrap();
            // If the terminal was just unzoomed, GTK is not yet aware of this when we call
            // compute_bounds(), which means we have to use the provided x and y coordinates now
            bounds.offset(-x, -y);
            // println!("Bounds are {:?}", bounds);
            let intersection = terminal_rect.intersection(&bounds);
            if intersection.is_some() {
                return Some(neighbor.clone());
            }
        }

        None
    }
}

pub fn create_tab(tab_view: &TabView, id: Option<u32>) {
    let tab_id = if let Some(id) = id {
        id
    } else {
        GLOBAL_TAB_ID.fetch_add(1, Ordering::Relaxed)
    };
    let top_level = TopLevel::new(tab_view);

    // Add pane as a page
    let page = tab_view.append(&top_level);

    let text = format!("Terminal {}", tab_id);
    page.set_title(&text);
    tab_view.set_selected_page(&page);
}
