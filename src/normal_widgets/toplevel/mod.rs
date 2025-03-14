mod imp;
mod layout;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use gtk4::{gdk::Rectangle, graphene::Rect, Orientation, Widget};
use libadwaita::{glib, prelude::*, TabView};

use crate::{
    config::SPLIT_HANDLE_WIDTH,
    helpers::{borrow_clone, WithId},
    keyboard::Direction,
    modals::spawn_rename_modal,
};

use self::imp::Zoomed;

use super::{container::Container, terminal::Terminal, window::IvyNormalWindow};

glib::wrapper! {
    pub struct TopLevel(ObjectSubclass<imp::TopLevelPriv>)
        @extends libadwaita::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Actionable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl TopLevel {
    pub fn new(tab_view: &TabView, window: &IvyNormalWindow, tab_id: u32) -> Self {
        let top_level: TopLevel = Object::builder().build();
        top_level.set_vexpand(true);
        top_level.set_hexpand(true);
        top_level.set_focusable(true);

        top_level.imp().init_values(tab_view, window, tab_id);

        let terminal = Terminal::new(&top_level, window, None);
        top_level.set_child(Some(&terminal));

        top_level
    }

    pub fn create_tab(&self) {
        let window = borrow_clone(&self.imp().window);
        window.new_tab();
    }

    pub fn close_tab(&self) {
        let window = self.imp().window.take();
        if let Some(window) = window {
            window.close_tab(self);
        }
    }

    pub fn tab_id(&self) -> u32 {
        self.imp().tab_id.get()
    }

    pub fn split_pane(
        &self,
        terminal: &Terminal,
        orientation: Orientation,
    ) -> (Terminal, Option<Container>) {
        self.unzoom();

        let window = borrow_clone(&self.imp().window);
        let new_terminal = Terminal::new(&self, &window, None);

        let parent = terminal.parent().unwrap();
        if parent.eq(self) {
            // Terminal's parent is myself
            let old_terminal = self.child().unwrap();

            self.set_child(None::<&Self>);
            let container = Container::new(orientation);
            container.append(&old_terminal);
            container.append(&new_terminal);
            self.set_child(Some(&container));
            return (new_terminal, Some(container));
        }

        // Terminal's parent is a Container widget
        let container: Container = parent.downcast().unwrap();

        // If the split orientation is the same as Container's orientation, we can simply insert a new Pane
        if container.orientation() == orientation {
            container.append(&new_terminal);
            return (new_terminal, None);
        }

        // The split orientation is different from Container's, meaning we have to insert a new Container
        let new_container = Container::new(orientation);
        container.replace(terminal, &new_container);
        new_container.append(terminal);
        new_container.append(&new_terminal);

        return (new_terminal, Some(new_container));
    }

    pub fn close_pane(&self, closing_terminal: &Terminal) {
        self.unzoom();
        self.unregister_terminal(closing_terminal);

        let parent = closing_terminal.parent().unwrap();
        if parent.eq(self) {
            // Parent of the closing terminal is myself, we need to close this tab
            self.close_tab();
            return;
        }

        // Parent of the closing terminal is a Container widget
        let container: Container = parent.downcast().unwrap();
        let remaining_count = container.remove(closing_terminal);

        // At this point we know there is at least 1 remaining terminal
        let last_focused_terminal = self.lru_terminal().unwrap();

        // If the conatiner has more than 1 child left, we are done. Otherwise remove the container
        // and leave the 1 child in its place.
        if remaining_count > 1 {
            last_focused_terminal.grab_focus();
            return;
        }

        // Remove the last child from Container, which will be deleted
        let retained_child = container.first_child().unwrap();
        retained_child.unparent();

        // Determine if parent is type Bin or Container
        let parent = container.parent().unwrap();

        // TODO: Swap TopLevel and Container (since Container is more common)
        let parent = match parent.downcast::<TopLevel>() {
            Ok(parent) => {
                // Parent is TopLevel
                parent.set_child(Some(&retained_child));
                // Since retained_child is the only terminal remaining, focus it
                last_focused_terminal.grab_focus();
                return;
            }
            // Fall-through
            Err(parent) => parent,
        };

        if let Ok(parent) = parent.downcast::<Container>() {
            // Parent is another Container
            parent.replace(&container, &retained_child);
            // Grab focus on the least recently used terminal
            last_focused_terminal.grab_focus();
            return;
        }

        panic!("Parent is neither Bin nor Container");
    }

    pub fn toggle_zoom(&self, terminal: &Terminal) {
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
            z.terminal
                .insert_after(&z.terminal_container, z.previous_sibling.as_ref());

            self.set_child(Some(&z.root_container));
            z.terminal.grab_focus();
            return;
        }
        // Zoom the terminal

        // We need to remember the current width and height for the unzoom portion
        let allocation = terminal.allocation();

        // Remove Terminal from its parent Container
        let container: Container = terminal.parent().unwrap().downcast().unwrap();
        let previous_sibling = terminal.prev_sibling();
        terminal.unparent();

        // Remove root Container and replace it with Terminal
        let root_container: Container = self.child().unwrap().downcast().unwrap();
        self.set_child(Some(terminal));
        terminal.grab_focus();

        let zoomed = Zoomed {
            terminal: terminal.clone(),
            root_container,
            terminal_container: container,
            previous_sibling,
            previous_bounds: allocation,
        };
        binding.replace(zoomed);
    }

    pub fn unzoom(&self) -> Option<Rectangle> {
        if let Some(z) = self.imp().zoomed.take() {
            self.set_child(None::<&Widget>);
            z.terminal
                .insert_after(&z.terminal_container, z.previous_sibling.as_ref());

            self.set_child(Some(&z.root_container));
            z.terminal.grab_focus();

            return Some(z.previous_bounds);
        }

        None
    }

    pub fn register_terminal(&self, terminal: &Terminal) {
        let pane_id = terminal.id();
        let imp = self.imp();

        let mut terminals_vec = imp.terminals.borrow_mut();
        terminals_vec.push(terminal.clone());

        let mut lru_terminals = imp.lru_terminals.borrow_mut();
        lru_terminals.insert(
            0,
            WithId {
                id: pane_id,
                terminal: terminal.clone(),
            },
        );

        // Also update global terminal hashmap
        let window = borrow_clone(&imp.window);
        window.register_terminal(pane_id, terminal);
    }

    pub fn unregister_terminal(&self, terminal: &Terminal) {
        let pane_id = terminal.id();
        let imp = self.imp();

        let mut terminals_vec = imp.terminals.borrow_mut();
        terminals_vec.retain(|t| t != terminal);
        drop(terminals_vec);

        let mut lru_terminals = imp.lru_terminals.borrow_mut();
        for (index, sorted) in lru_terminals.iter_mut().enumerate() {
            if sorted.id == pane_id {
                lru_terminals.remove(index);
                break;
            }
        }
        drop(lru_terminals);

        // Also update global terminal hashmap
        let window = borrow_clone(&imp.window);
        window.unregister_terminal(pane_id);
    }

    pub fn focus_changed(&self, id: u32, terminal: &Terminal) {
        let mut lru_terminals = self.imp().lru_terminals.borrow_mut();

        if let Some(id_terminal) = lru_terminals.first() {
            if id_terminal.id == id {
                // The LRU already has this terminal as latest, no need for any work
                return;
            }
        }

        // Remove the previous position in the vector
        for (index, sorted) in lru_terminals.iter_mut().enumerate() {
            if sorted.id == id {
                lru_terminals.remove(index);
                break;
            }
        }

        // Insert at the beginning
        lru_terminals.insert(
            0,
            WithId {
                id: id,
                terminal: terminal.clone(),
            },
        );
    }

    pub fn lru_terminal(&self) -> Option<Terminal> {
        let lru_terminals = self.imp().lru_terminals.borrow();
        match lru_terminals.first() {
            Some(id_terminal) => Some(id_terminal.terminal.clone()),
            None => None,
        }
    }

    pub fn find_neighbor(
        &self,
        terminal: &Terminal,
        direction: Direction,
        use_size: Option<Rectangle>,
    ) -> Option<Terminal> {
        let terminals = self.imp().terminals.borrow();
        if terminals.len() < 2 {
            return None;
        }

        const PAD: f32 = SPLIT_HANDLE_WIDTH as f32 + 5.0;

        // We will use Rect intersection to find a matching neighbor. For this to work, the Rect
        // used for calculating the intersection must be slightly larger in the direction we
        // wish to find a neighbor.
        let (x, y, width, height) = if let Some(allocation) = use_size {
            let x = allocation.x();
            let y = allocation.y();
            let width = allocation.width();
            let height = allocation.height();
            (x as f32, y as f32, width as f32, height as f32)
        } else {
            let allocation = terminal.allocation();
            let width = allocation.width();
            let height = allocation.height();
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
        for neighbor in terminals.iter() {
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

    pub fn open_rename_modal(&self) {
        // Get TabView Page first
        let imp = self.imp();
        let tab_view = borrow_clone(&imp.tab_view);
        // TODO: Just store the Page directly instead of tab_view
        let page = tab_view.page(self);
        let current_name = page.title();

        let callback = glib::closure_local!(
            #[weak(rename_to = top_level)]
            self,
            move |new_name: &str| {
                top_level
                    .imp()
                    .name
                    .borrow_mut()
                    .replace(new_name.to_string());
                page.set_title(new_name);
            }
        );

        // We need the "parent" Window for modal
        let parent = borrow_clone(&imp.window);
        spawn_rename_modal(parent.upcast_ref(), &current_name, callback);
    }

    pub fn select_tab(&self, previous: bool) {
        let imp = self.imp();
        let tab_view = borrow_clone(&imp.tab_view);

        if previous {
            tab_view.select_previous_page();
        } else {
            tab_view.select_next_page();
        }
    }

    pub fn terminal_title_changed(&self, name: &str) {
        let imp = self.imp();

        if imp.name.borrow().is_some() {
            return;
        }

        // We ditch the user@host:... part of the title
        if let Some(name) = name.split(':').last() {
            let tab_view = borrow_clone(&imp.tab_view);
            // TODO: Just store the Page directly instead of tab_view
            let page = tab_view.page(self);
            page.set_title(name);
        }
    }
}
