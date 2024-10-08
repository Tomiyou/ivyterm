use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::{Orientation, Widget};
use libadwaita::prelude::*;

use crate::{
    tmux_api::{Rectangle, TmuxPane},
    tmux_widgets::{
        container::{TmuxContainer, TmuxSeparator},
        terminal::TmuxTerminal,
        toplevel::TmuxTopLevel,
    },
};

use super::IvyTmuxWindow;

struct ParentContainer {
    c: TmuxContainer,
    bounds: Rectangle,
}

fn print_tab(nested: u32) {
    for _ in 0..nested {
        print!("    ");
    }
}

pub fn sync_tmux_layout(window: &IvyTmuxWindow, tab_id: u32, layout: Vec<TmuxPane>) {
    let window_imp = window.imp();

    {
        let mut nested = 0;
        for pane in layout.iter() {
            match pane {
                TmuxPane::Container(_, _) => {
                    print_tab(nested);
                    println!("- {:?}", pane);
                    nested += 1
                }
                TmuxPane::Return => nested -= 1,
                _ => {
                    print_tab(nested);
                    println!("- {:?}", pane);
                }
            }
        }
    }

    let top_level = if let Some(top_level) = window.get_top_level(tab_id) {
        println!("Reusing top Level {}", top_level.tab_id());
        top_level
    } else {
        println!("Creating new Tab (with new top_level)");
        window.new_tab(tab_id)
    };

    // Print hierarchy for debug purposes
    if log::log_enabled!(log::Level::Debug) {
        print_hierarchy(&top_level, 0);
    }

    // First we remove any Terminals which do not exist in Tmux anymore
    // TODO: Make this less brute force
    {
        let mut registered_terminals = window_imp.terminals.borrow_mut();
        let original_len = registered_terminals.len();

        registered_terminals.retain(|t| {
            let mut still_exists = false;
            // Check if our registered terminal has NOT been closed
            for pane in layout.iter() {
                match pane {
                    TmuxPane::Terminal(pane_id, _) => {
                        if t.id == *pane_id {
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
            println!("Terminal {} closed by Tmux", t.id);

            let parent = t.terminal.parent();
            remove_pane(&t.terminal);

            if original_len > 1 {
                // We know that there is at least 1 TmuxContainer, so Parent must be Container
                if let Some(container) = parent {
                    // If Container has only 1 child left at this point, we should remove it
                    let first_child = container.first_child().unwrap();
                    let last_child = container.last_child().unwrap();
                    if first_child.eq(&last_child) {
                        // First child is also the last child (only 1 child left)
                        println!("Leftover child replacing closing Container");
                        first_child.unparent();
                        replace_pane(&container, &first_child);
                    }
                }
            }

            false
        });
    }

    // All closed Terminals are gone at this point
    // Now we have to determine if the first child is a Pane or a Container
    let mut iter = layout.iter();
    if let Some(first) = iter.next() {
        match first {
            TmuxPane::Terminal(term_id, _) => {
                let term_id = *term_id;
                // terminal_callback(pane_id, window, top_level, parent, allocation, &mut current_sibling);
                if let Some(existing) = window.get_terminal_by_id(term_id) {
                    // Pane already exists
                    if let Some(child) = top_level.child() {
                        if existing.eq(&child) {
                            // Pane is already in the correct place, nothing to do
                            println!("Pane already correctly placed {}", term_id);
                        } else {
                            // Replace the current child with ourselves
                            top_level.set_child(Some(&existing));
                            println!("Pane {} replaced the only child", term_id);
                        }
                    } else {
                        // This is a very strange case, Terminal already exists, but top_level has
                        // not children???
                        eprintln!(
                            "Terminal {} already exists, but top_level has not children??",
                            term_id
                        );
                        top_level.set_child(Some(&existing));
                    }
                } else {
                    // Terminal doesn't exist yet, we need to create it
                    // Terminal does not exist yet, simply append it after previous_sibling
                    let new_terminal = TmuxTerminal::new(&top_level, window, term_id);
                    top_level.set_child(Some(&new_terminal));
                    println!("Created pane {} as only child", term_id);
                }
            }
            TmuxPane::Container(orientation, allocation) => {
                let container = if let Some(child) = top_level.child() {
                    if let Ok(container) = child.downcast::<TmuxContainer>() {
                        // The first child is already a Container
                        println!("The first child is already a Container");
                        container
                    } else {
                        // The first child is a Terminal, replace with a new Container
                        println!("The first child is a Terminal, replace with a new Container");
                        let container = TmuxContainer::new(orientation, window);
                        top_level.set_child(Some(&container));
                        container
                    }
                } else {
                    // top_level doesn't have any children yet
                    println!("top_level doesn't have any children yet");
                    let container = TmuxContainer::new(orientation, window);
                    top_level.set_child(Some(&container));
                    container
                };

                let container = ParentContainer {
                    c: container,
                    bounds: *allocation,
                };

                sync_layout_recursive(&mut iter, window, &top_level, &container, 1);
            }
            _ => {
                panic!("Parsed Layout has no Terminals")
            }
        }
    } else {
        panic!("Parsed Layout empty")
    }

    // TODO: Fix this, Tmux currently does not report active Pane
    // Ensure the correct Pane is focused
    let focused_pane = window_imp.focused_pane.get();
    let registered_terminals = window_imp.terminals.borrow();
    if let Some(terminal) = registered_terminals.get(focused_pane) {
        println!("Grabbing focus for pane {}", focused_pane);
        terminal.grab_focus();
    } else {
        println!("Unable to grab focus for pane {}", focused_pane);
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
                return;
            }
            TmuxPane::Terminal(term_id, allocation) => {
                print_tab(nested);
                println!("-- NEXT ITEM: {:?}", tmux_pane);

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
                print_tab(nested);
                println!("-- NEXT ITEM: {:?}", tmux_pane);

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
        print_tab(nested);
        println!("Unparenting child!!!");
        child.unparent();
        // We do this here to avoid cloning on downcast()
        current_sibling = child.next_sibling();

        // TODO: I think Widgets without any parent should recursively be destroyed on its own
        // DO CHECK THIS!!
        // if let Ok(container) = child.downcast::<TmuxContainer>() {
        //     remove_unparented_widgets(&container);
        // }
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

    // If the next_sibling is already a Container, we don't have to create it
    if let Some(next_pane) = next_sibling {
        if let Ok(container) = next_pane.clone().downcast::<TmuxContainer>() {
            print_tab(nested);
            println!("Container is already in the correct place");
            if let Some(separator) = container.next_sibling() {
                let separator: TmuxSeparator = separator.downcast().unwrap();
                separator.set_position(position);
            }
            return container;
        } else {
            print_tab(nested);
            if let Ok(terminal) = next_pane.clone().downcast::<TmuxTerminal>() {
                println!(
                    "Creating new Container to replace the current child TERMINAL {}, position {}",
                    terminal.pane_id(),
                    position
                );
            } else {
                println!(
                    "Creating new Container to replace the current child (type {}), position {}",
                    next_pane.type_(),
                    position
                );
            }
        }
    } else {
        print_tab(nested);
        println!(
            "Creating new Container, next_sibling is None, position {}",
            position
        )
    }

    let container = TmuxContainer::new(&orientation, window);
    prepend_pane(window, &parent.c, &container, next_sibling, position);

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
                print_tab(nested);
                println!(
                    "Terminal with ID {} already in the correct place, position is {}",
                    pane_id, position
                );
                // Pane is in correct place, just make sure the Separator position is correct
                if let Some(separator) = next_pane.next_sibling() {
                    let separator: TmuxSeparator = separator.downcast().unwrap();
                    separator.set_position(position);
                }
                return existing;
            }
        }

        // The pane exists, but is not in the correct place, remove it from its
        // current position first
        remove_pane(&existing);
        // Now insert it in the correct place
        prepend_pane(window, &parent.c, &existing, next_sibling, position);
        print_tab(nested);
        println!(
            "Terminal with ID {} moved to new position ({})",
            pane_id, position
        );

        return existing;
    }

    // Terminal does not exist yet, simply prepend it before next_sibling
    print_tab(nested);
    let new_terminal = TmuxTerminal::new(top_level, window, pane_id);
    prepend_pane(window, &parent.c, &new_terminal, next_sibling, position);

    new_terminal
}

// TODO: Turn this into impl {} for Rectangle
#[inline]
fn calculate_position(bounds: &Rectangle, parent: &ParentContainer) -> i32 {
    let orientation = parent.c.orientation();
    // match orientation {
    //     Orientation::Horizontal => bounds.x - parent.bounds.x - 1,
    //     _ => bounds.y - parent.bounds.y - 1,
    // }

    match orientation {
        Orientation::Horizontal => bounds.width,
        _ => bounds.height,
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
    position: i32,
) {
    // TODO: Prepend on sibling None means append() last...
    if let Some(sibling) = sibling {
        let new_separator = create_separator(window, container, position);
        child.insert_before(container, Some(sibling));
        new_separator.insert_after(container, Some(child));
    } else {
        append_pane(window, container, child, position);
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

fn replace_pane(old: &impl IsA<Widget>, new: &impl IsA<Widget>) {
    let parent = old.parent().unwrap();
    new.insert_after(&parent, Some(old));
    old.unparent();
}

fn print_hierarchy(widget: &impl IsA<Widget>, nested: u32) {
    let widget = widget.as_ref();
    let mut nested = nested;
    if let Ok(container) = widget.clone().downcast::<TmuxContainer>() {
        print_tab(nested);
        println!("** Container {}", container.type_());
        nested += 1;
    } else if let Ok(terminal) = widget.clone().downcast::<TmuxTerminal>() {
        print_tab(nested);
        println!("** Terminal ({}) {}", terminal.pane_id(), terminal.type_());
        nested += 1;
    } else if let Ok(separator) = widget.clone().downcast::<TmuxSeparator>() {
        print_tab(nested);
        println!("** Separator {}", separator.type_());
        nested += 1;
    }
    let mut next_child = widget.first_child();
    while let Some(child) = &next_child {
        print_hierarchy(child, nested);
        next_child = child.next_sibling();
    }
}
