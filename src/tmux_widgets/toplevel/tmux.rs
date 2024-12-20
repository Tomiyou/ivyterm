use super::{imp::Zoomed, TmuxTopLevel};

use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::{Orientation, Widget};
use libadwaita::prelude::*;
use log::debug;

use crate::{
    tmux_api::{LayoutFlags, LayoutSync, Rectangle, TmuxPane},
    tmux_widgets::{container::TmuxContainer, separator::TmuxSeparator, terminal::TmuxTerminal},
};

use super::IvyTmuxWindow;

struct ParentContainer {
    c: TmuxContainer,
    bounds: Rectangle,
}

#[inline]
fn print_tab(nested: u32) {
    for _ in 0..nested {
        print!("    ");
    }
}

#[inline]
fn print_tab_debug(nested: u32) {
    if log::log_enabled!(log::Level::Debug) {
        for _ in 0..nested {
            debug!("    ");
        }
    }
}

impl TmuxTopLevel {
    fn close_removed_terminals(&self, window: &IvyTmuxWindow, layout: &Vec<TmuxPane>) {
        let mut registered_terminals = self.imp().terminals.borrow_mut();
        let original_len = registered_terminals.len();

        // TODO: Make this less brute force
        registered_terminals.retain(|terminal| {
            let term_id = terminal.id();

            let mut still_exists = false;
            // Check if our registered terminal has NOT been closed
            for pane in layout.iter() {
                match pane {
                    TmuxPane::Terminal(pane_id, _) => {
                        if term_id == *pane_id {
                            still_exists = true;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if still_exists {
                return true;
            }

            // Terminal has been closed by Tmux, we have to do the same
            debug!("Terminal {} closed by Tmux", term_id);

            let parent = terminal.parent();
            remove_pane(terminal);

            if original_len > 1 {
                // We know that there is at least 1 TmuxContainer, so Parent must be Container
                if let Some(container) = parent {
                    // If Container has only 1 child left at this point, we should remove it
                    let first_child = container.first_child().unwrap();
                    let last_child = container.last_child().unwrap();
                    if first_child.eq(&last_child) {
                        // First child is also the last child (only 1 child left)
                        debug!(
                            "Leftover child {} replacing closing Container {}",
                            first_child.type_(),
                            container.type_()
                        );
                        replace_container(&container, &first_child);
                    }
                }
            }

            // Terminal is unregistered for TopLevel, but we also need to unregister it from Window
            window.unregister_terminal(term_id);

            false
        });
    }

    #[inline]
    fn handle_zoomed_terminal(
        &self,
        window: &IvyTmuxWindow,
        visible_layout: &Vec<TmuxPane>,
    ) -> Zoomed {
        // Check that visible_layout is not empty
        let pane = match visible_layout.first() {
            Some(pane) => pane,
            None => panic!("Tab is zoomed, but hierarchy is empty"),
        };

        // Check that the pane is actually a Terminal
        let term_id = match pane {
            TmuxPane::Terminal(term_id, _) => *term_id,
            _ => panic!("Tab is zoomed, but hierarchy doesn't match that"),
        };

        let terminal = match window.get_terminal_by_id(term_id) {
            Some(terminal) => terminal,
            None => panic!("Terminal {} zoomed by Tmux, but cannot be found!", term_id),
        };

        self.zoom(term_id, terminal)
    }

    pub fn sync_tmux_layout(&self, window: &IvyTmuxWindow, layout_sync: LayoutSync) {
        let layout = layout_sync.layout;

        if log::log_enabled!(log::Level::Debug) {
            let mut nested = 0;
            for pane in layout.iter() {
                match pane {
                    TmuxPane::Container(_, _) => {
                        print_tab(nested);
                        debug!("- {:?}", pane);
                        nested += 1
                    }
                    TmuxPane::Return => nested -= 1,
                    _ => {
                        print_tab(nested);
                        debug!("- {:?}", pane);
                    }
                }
            }
        }

        // Print hierarchy for debug purposes
        if log::log_enabled!(log::Level::Debug) {
            print_hierarchy(self, 0);
        }

        if let Some(name) = layout_sync.name {
            self.tab_renamed(&name);
        }

        // First Unzoom (we AWLAYS unzoom to handle corner cases better)
        let imp = self.imp();
        if let Some(zoomed) = imp.zoomed.take() {
            self.unzoom(zoomed);
        }

        // First we remove any Terminals which do not exist in Tmux anymore
        self.close_removed_terminals(window, &layout);

        // All closed Terminals are gone at this point
        // Now we have to determine if the first child is a Pane or a Container
        let mut iter = layout.iter();
        if let Some(first) = iter.next() {
            match first {
                TmuxPane::Terminal(term_id, _) => {
                    let term_id = *term_id;
                    // terminal_callback(pane_id, window, self, parent, allocation, &mut current_sibling);
                    if let Some(existing) = window.get_terminal_by_id(term_id) {
                        // Pane already exists
                        if let Some(child) = self.child() {
                            if existing.eq(&child) {
                                // Pane is already in the correct place, nothing to do
                                debug!("Pane already correctly placed {}", term_id);
                            } else {
                                // Replace the current child with ourselves
                                self.set_child(Some(&existing));
                                debug!("Pane {} replaced the only child", term_id);
                            }
                        } else {
                            // This is a very strange case, Terminal already exists, but top_level has
                            // not children???
                            eprintln!(
                                "Terminal {} already exists, but top_level has not children??",
                                term_id
                            );
                            self.set_child(Some(&existing));
                        }
                    } else {
                        // Terminal doesn't exist yet, we need to create it
                        // Terminal does not exist yet, simply append it after previous_sibling
                        let new_terminal = TmuxTerminal::new(self, window, term_id);
                        self.set_child(Some(&new_terminal));
                        self.select_terminal_event(term_id);
                        debug!("Created pane {} as only child", term_id);
                    }
                }
                TmuxPane::Container(orientation, allocation) => {
                    let container = if let Some(child) = self.child() {
                        if let Ok(container) = child.downcast::<TmuxContainer>() {
                            // The first child is already a Container
                            debug!("The first child is already a Container");
                            container
                        } else {
                            // The first child is a Terminal, replace with a new Container
                            debug!("The first child is a Terminal, replace with a new Container");
                            let container = TmuxContainer::new(orientation, window);
                            self.set_child(Some(&container));
                            container
                        }
                    } else {
                        // top_level doesn't have any children yet
                        debug!("top_level doesn't have any children yet");
                        let container = TmuxContainer::new(orientation, window);
                        self.set_child(Some(&container));
                        container
                    };

                    let container = ParentContainer {
                        c: container,
                        bounds: *allocation,
                    };

                    sync_layout_recursive(&mut iter, window, self, &container, 1);
                }
                _ => {
                    panic!("Parsed Layout has no Terminals")
                }
            }
        } else {
            panic!("Parsed Layout empty")
        }

        // Now we can zoom Terminal
        if layout_sync.flags.contains(LayoutFlags::IsZoomed) {
            let zoomed = self.handle_zoomed_terminal(window, &layout_sync.visible_layout);
            imp.zoomed.replace(Some(zoomed));
        }

        self.focus_current_terminal();
    }
}

#[inline]
fn move_sibling_pointer(pointer: &mut Option<Widget>, widget: &impl IsA<Widget>) {
    *pointer = widget.next_sibling();

    // We just moved the pointer and if the pointer is now None, we are done
    // However, if that pointer is now Some(), it must be pointing to an instance
    // of TmuxSeparator (assuming our code is correct)
    if let Some(separator) = pointer {
        separator
            .clone()
            .downcast::<TmuxSeparator>()
            .expect("After moving the pointer sibling MUST be a TmuxSeparator (but it is NOT!)");
        *pointer = separator.next_sibling();
    }
}

fn sync_layout_recursive(
    layout: &mut std::slice::Iter<TmuxPane>,
    window: &IvyTmuxWindow,
    top_level: &TmuxTopLevel,
    parent: &ParentContainer,
    nested: u32,
) {
    let mut current_sibling = parent.c.first_child();

    // Walk list of children, keeping track of the current one
    // After all the input has be processed, destroy any unparented Terminals
    // Callback function should act on that existing child, depending on what
    // input is given:
    // -- Terminal is given:
    //    ** if ID does not match or existing child is a Container, we need to insert
    //       this given Terminal before existing child - make sure we check if the
    //       Terminal already exists
    //    ** otherwise we simply update the Terminal size
    // -- Container is given:
    //    ** if the current child is not already a Container, insert a new Container
    //    ALWAYS: and descend recursively

    while let Some(tmux_pane) = layout.next() {
        match tmux_pane {
            TmuxPane::Return => {
                break;
            }
            TmuxPane::Terminal(term_id, allocation) => {
                debug!("-- NEXT ITEM: {:?}", tmux_pane);

                let terminal = terminal_callback(
                    *term_id,
                    window,
                    top_level,
                    parent,
                    allocation,
                    &current_sibling,
                    nested,
                );
                move_sibling_pointer(&mut current_sibling, &terminal);
            }
            TmuxPane::Container(orientation, allocation) => {
                print_tab_debug(nested);
                debug!("-- NEXT ITEM: {:?}", tmux_pane);

                let container = container_callback(
                    orientation,
                    window,
                    parent,
                    allocation,
                    &current_sibling,
                    nested,
                );

                // Recursively call into TmuxContainer
                let container = ParentContainer {
                    c: container,
                    bounds: *allocation,
                };
                sync_layout_recursive(layout, window, top_level, &container, nested + 1);

                // Hierarchy might changed underneath us, move the pointer NOW
                move_sibling_pointer(&mut current_sibling, &container.c);
            }
        }
    }

    // Unparent all siblings we have left (since Tmux session obviously doesn't have them here)
    while let Some(child) = current_sibling {
        // We do this here to avoid cloning on downcast()
        current_sibling = child.next_sibling();

        print_tab_debug(nested);
        debug!("Unparenting child!!!");
        child.unparent();

        // TODO: I think Widgets without any parent should recursively be destroyed on its own
        // DO CHECK THIS!!
        // if let Ok(container) = child.downcast::<TmuxContainer>() {
        //     remove_unparented_widgets(&container);
        // }
    }

    parent.c.queue_allocate();
}

struct Position {
    next: i32,
    prev: i32,
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Customize so only `x` and `y` are denoted.
        write!(f, "Position {{ next: {}, prev: {} }}", self.next, self.prev)
    }
}

/// fn container_callback()
///
///     Handles Terminal layout
///
/// ** if ID does not match or existing child is a Container, we need to insert
///    this given Terminal before existing child - make sure we check if the
///    Terminal already exists
///
/// ** otherwise we simply update the Terminal size
#[inline]
fn container_callback(
    orientation: &Orientation,
    window: &IvyTmuxWindow,
    parent: &ParentContainer,
    bounds: &Rectangle,
    next_sibling: &Option<Widget>,
    nested: u32,
) -> TmuxContainer {
    let position = calculate_position(&bounds, parent);

    // Check if next_sibling even exists
    if let Some(next_pane) = next_sibling {
        // If the next_sibling is already a Container, we don't have to create it
        let next_pane = next_pane.clone();
        let next_pane = match next_pane.downcast::<TmuxContainer>() {
            Ok(container) => {
                print_tab_debug(nested);
                debug!("Container is already in the correct place");
                if let Some(separator) = container.next_sibling() {
                    let separator: TmuxSeparator = separator.downcast().unwrap();
                    separator.set_position(position.next);
                }
                return container;
            }
            Err(next_pane) => next_pane,
        };

        // next_sibling is NOT a Container, print some info to log
        if log::log_enabled!(log::Level::Debug) {
            print_tab_debug(nested);
            match next_pane.downcast::<TmuxTerminal>() {
                Ok(terminal) => {
                    debug!(
                        "Creating new Container to replace the current child TERMINAL {}, position {}",
                        terminal.id(),
                        position
                    );
                }
                Err(next_pane) => {
                    debug!(
                        "Creating new Container to replace the current child (type {}), position {}",
                        next_pane.type_(),
                        position
                    );
                }
            }
        }
    } else {
        print_tab_debug(nested);
        debug!(
            "Creating new Container, next_sibling is None, position {}",
            position
        )
    }

    let container = TmuxContainer::new(&orientation, window);
    prepend_pane(window, &parent.c, &container, next_sibling, &position);

    container
}

// Use one for Pane and one for Container
#[inline]
fn terminal_callback(
    pane_id: u32,
    window: &IvyTmuxWindow,
    top_level: &TmuxTopLevel,
    parent: &ParentContainer,
    bounds: &Rectangle,
    next_sibling: &Option<Widget>,
    nested: u32,
) -> TmuxTerminal {
    // We know Terminal with given pane_id should be exactly *here* (as in before/exactly next_sibling)
    // next_sibling is always either Terminal or Container
    let position = calculate_position(&bounds, parent);

    // Check if a terminal with the given pane_id already exists
    if let Some(existing) = window.get_terminal_by_id(pane_id) {
        // Check if there is a next_sibling
        if let Some(next_pane) = next_sibling {
            // Check if this next_pane is already this terminal
            if existing.eq(next_pane) {
                print_tab_debug(nested);
                debug!(
                    "Terminal with ID {} already in the correct place, position is {}",
                    pane_id, position.next
                );
                // Pane is in correct place, just make sure the Separator position is correct
                if let Some(separator) = next_pane.next_sibling() {
                    let separator: TmuxSeparator = separator.downcast().unwrap();
                    separator.set_position(position.next);
                }
                return existing;
            }
        }

        // The pane exists, but is not in the correct place, remove it from its
        // current position first
        remove_pane(&existing);
        // Now insert it in the correct place
        prepend_pane(window, &parent.c, &existing, next_sibling, &position);
        print_tab_debug(nested);
        debug!(
            "Terminal with ID {} moved to new position ({})",
            pane_id, position
        );

        return existing;
    }

    // Terminal does not exist yet, simply prepend it before next_sibling
    print_tab_debug(nested);
    let new_terminal = TmuxTerminal::new(top_level, window, pane_id);
    print_tab_debug(nested);
    debug!("   \\---> position {}", position);
    prepend_pane(window, &parent.c, &new_terminal, next_sibling, &position);

    new_terminal
}

// TODO: Turn this into impl {} for Rectangle
#[inline]
fn calculate_position(bounds: &Rectangle, parent: &ParentContainer) -> Position {
    let orientation = parent.c.orientation();

    // Depending if widget is last or not is why we need BOTH a position for
    // a next_sibling() Separator and a prev_sibling() Separator
    match orientation {
        Orientation::Horizontal => Position {
            prev: bounds.x - parent.bounds.x - 1,
            next: bounds.x - parent.bounds.x + bounds.width,
        },
        _ => Position {
            prev: bounds.y - parent.bounds.y - 1,
            next: bounds.y - parent.bounds.y + bounds.height,
        },
    }
}

#[allow(dead_code)]
fn remove_unparented_widgets(container: &TmuxContainer) {
    let mut next_child = container.first_child();

    while let Some(child) = next_child {
        child.unparent();
        // We do this here to avoid cloning on downcast()
        next_child = child.next_sibling();
        if let Ok(container) = child.downcast::<TmuxContainer>() {
            remove_unparented_widgets(&container);
        }
    }
}

#[inline]
fn create_separator(
    window: &IvyTmuxWindow,
    container: &TmuxContainer,
    position: i32,
) -> TmuxSeparator {
    let (char_width, char_height) = window.get_char_size();
    let orientation = container.orientation();

    let handle_size = match orientation {
        Orientation::Horizontal => char_width,
        _ => char_height,
    };

    TmuxSeparator::new(&orientation, handle_size, position)
}

fn append_pane(
    window: &IvyTmuxWindow,
    container: &TmuxContainer,
    child: &impl IsA<Widget>,
    position: i32,
) {
    if let Some(last_child) = container.last_child() {
        let new_separator = create_separator(window, container, position);
        new_separator.insert_after(container, Some(&last_child));
        child.insert_after(container, Some(&new_separator));
    } else {
        child.insert_after(container, None::<&Widget>);
    }
}

fn prepend_pane(
    window: &IvyTmuxWindow,
    container: &TmuxContainer,
    child: &impl IsA<Widget>,
    sibling: &Option<impl IsA<Widget>>,
    position: &Position,
) {
    // TODO: Prepend on sibling None means append() last...
    if let Some(sibling) = sibling {
        let new_separator = create_separator(window, container, position.next);
        child.insert_before(container, Some(sibling));
        new_separator.insert_after(container, Some(child));
    } else {
        append_pane(window, container, child, position.prev);
    }
}

fn remove_pane(child: &impl IsA<Widget>) {
    // First try and remove the associated separator
    if let Some(separator) = child.next_sibling() {
        separator.unparent();
    } else if let Some(separator) = child.prev_sibling() {
        separator.unparent();
    }

    // Now remove the child
    child.unparent();
}

fn replace_container(closing: &impl IsA<Widget>, survivor: &impl IsA<Widget>) {
    // At this point, closing is parent of survivor
    survivor.unparent();

    let parent = closing.parent().unwrap();

    // TODO: use match for all castings
    match parent.downcast::<TmuxTopLevel>() {
        Ok(top_level) => {
            debug!("Using set_child() instead of unparent() ...");
            top_level.set_child(Some(survivor));
        }
        Err(container) => {
            survivor.insert_after(&container, Some(closing));
            closing.unparent();
        }
    }
}

fn print_hierarchy(widget: &impl IsA<Widget>, nested: u32) {
    let mut nested = nested;

    if widget.is::<TmuxContainer>() {
        print_tab(nested);
        debug!("** Container {}", widget.type_());
        nested += 1;
    } else if widget.is::<TmuxTerminal>() {
        print_tab(nested);
        let terminal: TmuxTerminal = widget.as_ref().clone().downcast().unwrap();
        debug!("** Terminal ({}) {}", terminal.id(), terminal.type_());
        nested += 1;
    } else if widget.is::<TmuxSeparator>() {
        print_tab(nested);
        debug!("** Separator {}", widget.type_());
        nested += 1;
    }

    let mut next_child = widget.first_child();
    while let Some(child) = &next_child {
        print_hierarchy(child, nested);
        next_child = child.next_sibling();
    }
}
