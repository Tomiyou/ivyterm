use std::str::from_utf8;

use gtk4::{Orientation, Widget};
use libadwaita::prelude::*;

use crate::{container::{Container, TmuxLayout}, terminal::Terminal, toplevel::TopLevel, window::IvyWindow};

#[derive(Debug)]
struct Rectangle {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

fn print_tab(nested: u32) {
    for i in 0..nested {
        print!("    ");
    }
}

pub fn parse_tmux_layout(buffer: &[u8], window: &IvyWindow) {
    // Read tab ID
    let (tab_id, bytes_read) = read_first_u32(buffer);
    let buffer = &buffer[bytes_read + 1..];

    // Skip the initial whatever
    let bytes_read = read_until_char(buffer, b',');
    let buffer = &buffer[bytes_read + 1..];

    // Either get the existing tab, or spawn a new one if it does not exist yet
    let top_level = if let Some(top_level) = window.get_top_level(tab_id) {
        println!("Reusing top Level {}", top_level.tab_id());
        top_level
    } else {
        println!("Creating new Tab (with new top_level)");
        window.new_tab(Some(tab_id))
    };

    println!(
        "Tab id is {}, remaining buffer: {}",
        tab_id,
        from_utf8(buffer).unwrap()
    );

    // We parse the first level of layout separately (much more simple to implement this way)
    // More verbose, but simpler code (and more robust)
    parse_layout_root(buffer, window, &top_level);
}

struct TmuxContainer {
    c: Container,
    layout: TmuxLayout,
    bounds: Rectangle,
}

fn parse_layout_root(
    buffer: &[u8],
    window: &IvyWindow,
    top_level: &TopLevel,
) {
    let mut buffer = buffer;

    // Read width
    let (width, bytes_read) = read_first_u32(buffer);
    buffer = &buffer[bytes_read + 1..];

    // Read height
    let (height, bytes_read) = read_first_u32(buffer);
    buffer = &buffer[bytes_read + 1..];

    // Read x coordinate
    let (x, bytes_read) = read_first_u32(buffer);
    buffer = &buffer[bytes_read + 1..];

    // Read y coordinate
    let (y, bytes_read) = read_first_u32(buffer);
    buffer = &buffer[bytes_read..];

    let allocation = Rectangle {
        x: x as i32,
        y: y as i32,
        width: width as i32,
        height: height as i32,
    };

    // Now we have to determine if this is a Pane or a Container
    if buffer[0] == ',' as u8 {
        // This is a Pane
        buffer = &buffer[1..];

        let (pane_id, bytes_read) = read_first_u32(buffer);

        // terminal_callback(pane_id, window, top_level, parent, allocation, &mut current_sibling);
        if let Some(existing) = window.get_pane(pane_id) {
            // Pane already exists
            if let Some(child) = top_level.child() {
                if existing.eq(&child) {
                    // Pane is already in the correct place, nothing to do
                    println!("Pane already correctly placed {}", pane_id);
                } else {
                    // Replace the current child with ourselves
                    top_level.set_child(Some(&existing));
                    println!("Pane {} replaced the only child", pane_id);
                }
            } else {
                // This is a very strange case, Terminal already exists, but top_level has
                // not children???
                eprintln!("Terminal {} already exists, but top_level has not children??", pane_id);
                top_level.set_child(Some(&existing));
            }
        } else {
            // Terminal doesn't exist yet, we need to create it
            // Terminal does not exist yet, simply append it after previous_sibling
            let new_terminal = Terminal::new(top_level, window, Some(pane_id));
            top_level.set_child(Some(&new_terminal));
            println!("Created pane {} as only child", pane_id);
        }
    } else {
        // This is a Container
        let (orientation, open, close) = if buffer[0] == '[' as u8 {
            (Orientation::Vertical, b'[', b']')
        } else {
            (Orientation::Horizontal, b'{', b'}')
        };

        let container = if let Some(child) = top_level.child() {
            if let Ok(container) = child.downcast::<Container>() {
                println!("The first child is already a Container");
                // The first child is already a Container
                container
            } else {
                // The first child is a Terminal, replace with a new Container
                println!("The first child is a Terminal, replace with a new Container");
                top_level.set_child(None::<&Widget>);
                let container = Container::new(orientation, window);
                top_level.set_child(Some(&container));
                container
            }
        } else {
            // top_level doesn't have any children yet
            println!("top_level doesn't have any children yet");
            let container = Container::new(orientation, window);
            top_level.set_child(Some(&container));
            container
        };

        let layout: TmuxLayout = container.layout_manager().unwrap().downcast().unwrap();
        let container = TmuxContainer {
            c: container,
            layout,
            bounds: allocation,
        };

        // recursively call parse_tmux_layout
        let bytes_read = find_closing_bracket(buffer, open, close);
        parse_layout_recursive(
            &buffer[1..bytes_read],
            window,
            top_level,
            &container,
            1,
        );
    }

    top_level.unregister_unparented_terminals();
}

// @0,6306,80x5,0,0[80x2,0,0,0,80x2,0,3{40x2,0,3,1,39x2,41,3,2}]
fn parse_layout_recursive(
    buffer: &[u8],
    window: &IvyWindow,
    top_level: &TopLevel,
    parent: &TmuxContainer,
    nested: u32,
) {
    // We can assume that layout is purse ASCII text
    let mut buffer = buffer;
    let mut current_sibling = parent.c.first_child();

    print_tab(nested);
    println!("parse_layout_recursive: {}", from_utf8(buffer).unwrap());

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

    loop {
        // print_tab(nested);
        // println!("Remaining buffer: |{}|", from_utf8(buffer).unwrap());

        // Read width
        let (width, bytes_read) = read_first_u32(buffer);
        buffer = &buffer[bytes_read + 1..];

        // Read height
        let (height, bytes_read) = read_first_u32(buffer);
        buffer = &buffer[bytes_read + 1..];

        // Read x coordinate
        let (x, bytes_read) = read_first_u32(buffer);
        buffer = &buffer[bytes_read + 1..];

        // Read y coordinate
        let (y, bytes_read) = read_first_u32(buffer);
        buffer = &buffer[bytes_read..];

        let allocation = Rectangle {
            x: x as i32,
            y: y as i32,
            width: width as i32,
            height: height as i32,
        };

        // Now we have to determine if this is a Pane or a Container
        if buffer[0] == ',' as u8 {
            // This is a Pane
            buffer = &buffer[1..];

            let (pane_id, bytes_read) = read_first_u32(buffer);
            buffer = &buffer[bytes_read..];
            print_tab(nested);
            println!("-- Parsed Pane {} with size {:?}", pane_id, allocation);

            print_tab(nested);
            terminal_callback(pane_id, window, top_level, parent, allocation, &mut current_sibling, nested);
        } else {
            // This is a Container
            let (orientation, open, close) = if buffer[0] == '[' as u8 {
                (Orientation::Vertical, b'[', b']')
            } else {
                (Orientation::Horizontal, b'{', b'}')
            };
            print_tab(nested);
            println!("-- Parsed {:?} Container with size {:?}", orientation, allocation);

            print_tab(nested);
            let new_container = container_callback(
                orientation,
                window,
                top_level,
                parent,
                allocation,
                &mut current_sibling,
                nested,
            );

            // recursively call parse_tmux_layout
            let bytes_read = find_closing_bracket(buffer, open, close);
            parse_layout_recursive(
                &buffer[1..bytes_read],
                window,
                top_level,
                &new_container,
                nested + 1,
            );

            buffer = &buffer[bytes_read + 1..];
        }

        if buffer.is_empty() {
            break;
        }

        buffer = &buffer[1..];
    }

    // Unparent all siblings we have left (since Tmux session obviously doesn't have them here)
    while let Some(child) = current_sibling {
        print_tab(nested);
        println!("Unparenting child!!!");
        child.unparent();
        // We do this here to avoid cloning on downcast()
        current_sibling = child.next_sibling();
        if let Ok(container) = child.downcast::<Container>() {
            remove_unparented_widgets(&container);
        }
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
    orientation: Orientation,
    window: &IvyWindow,
    top_level: &TopLevel,
    parent: &TmuxContainer,
    bounds: Rectangle,
    next_sibling: &mut Option<Widget>,
    nested: u32,
) -> TmuxContainer {
    let position = calculate_position(&bounds, parent);

    // If the next_sibling is already a Container, we don't have to create it
    if let Some(next_pane) = next_sibling {
        if let Ok(container) = next_pane.clone().downcast::<Container>() {
            // Ensure bounds (position) are correct?
            println!("Container is already in the correct place");
            if let Some(separator) = container.next_sibling() {
                print_tab(nested);
                parent.layout.set_separator_position(&separator, position);
            }
            let layout: TmuxLayout = container.layout_manager().unwrap().downcast().unwrap();
            move_child_pointer(next_sibling, container.clone().upcast());
            return TmuxContainer {
                c: container,
                layout,
                bounds,
            };
        } else {
            println!("Created new Container to replace the current child, position {}", position);
        }
    } else {
        println!("Created new Container, next_sibling is None, position {}", position)
    }

    let container = Container::new(orientation, window);
    let layout: TmuxLayout = container.layout_manager().unwrap().downcast().unwrap();
    parent.c.prepend(&container, next_sibling, Some(position));

    return TmuxContainer {
        c: container,
        layout,
        bounds,
    };
}

// Use one for Pane and one for Container
#[inline]
fn terminal_callback(
    pane_id: u32,
    window: &IvyWindow,
    top_level: &TopLevel,
    parent: &TmuxContainer,
    bounds: Rectangle,
    next_sibling: &mut Option<Widget>,
    nested: u32,
) -> Terminal {
    // We know Terminal with given pane_id should be exactly *here* (as in before/exactly next_sibling)
    // next_sibling is always either Terminal or Container
    let position = calculate_position(&bounds, parent);

    // Check if a terminal with the given pane_id already exists
    if let Some(existing) = window.get_pane(pane_id) {
        // Check if there is a next_sibling
        if let Some(next_pane) = next_sibling {
            // Check if this next_pane is already this terminal
            if existing.eq(next_pane) {
                println!("Pane with ID {} already in the correct place, position is {}", pane_id, position);
                // Pane is in correct place, just make sure the Separator position is correct
                if let Some(separator) = next_pane.next_sibling() {
                    print_tab(nested);
                    parent.layout.set_separator_position(&separator, position);
                }
                // Since we skipped prepending a Terminal, we have to move the next_sibling pointer
                move_child_pointer(next_sibling, existing.clone().upcast());
                return existing;
            }
        }

        // The pane exists, but is not in the correct place, remove it from its
        // current position first
        unparent_pane(&existing);
        // Now insert it in the correct place
        parent.c.prepend(&existing, next_sibling, Some(position));
        println!("Pane with ID {} moved to new position ({})", pane_id, position);

        return existing;
    }

    // Terminal does not exist yet, simply append it after previous_sibling
    // print_tab(nested);
    let new_terminal = Terminal::new(top_level, window, Some(pane_id));
    parent.c.prepend(&new_terminal, next_sibling, Some(position));
    // println!("Created new Terminal {}", pane_id);

    new_terminal
}

// @0,6306,80x5,0,0[80x2,0,0,0,80x2,0,3{40x2,0,3,1,39x2,41,3,2}]
#[inline]
fn calculate_position(bounds: &Rectangle, parent: &TmuxContainer) -> i32 {
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

fn unparent_pane(pane: &impl IsA<Widget>) {
    // If there is a next_sibling, it must be Separator
    if let Some(separator) = pane.next_sibling() {
        separator.unparent();
    // If there was not a next_sibling, there could be a prev_sibling (we are the last pane)
    } else if let Some(separator) = pane.prev_sibling() {
        separator.unparent();
    }
    pane.unparent();
}

#[inline]
fn move_child_pointer(next_sibling: &mut Option<Widget>, new_child: Widget) {
    // If the new_child has a Separator following it, we need to point to it instead
    if let Some(separator) = new_child.next_sibling() {
        *next_sibling = Some(separator.next_sibling().unwrap());
        println!("Moving pointer AFTER Separator");
    } else {
        *next_sibling = None;
        println!("Moving pointer to None");
    }
}

fn remove_unparented_widgets(container: &Container) {
    let mut next_child = container.first_child();

    while let Some(child) = next_child {
        child.unparent();
        // We do this here to avoid cloning on downcast()
        next_child = child.next_sibling();
        if let Ok(container) = child.downcast::<Container>() {
            remove_unparented_widgets(&container);
        }
    }
}

#[inline]
pub fn read_first_u32(buffer: &[u8]) -> (u32, usize) {
    let mut i = 0;
    let mut number: u32 = 0;

    // Read buffer char by char (assuming ASCII) and parse number
    while i < buffer.len() && buffer[i] > 47 && buffer[i] < 58 {
        number *= 10;
        number += (buffer[i] - 48) as u32;
        i += 1;
    }
    (number, i)
}

#[inline]
pub fn read_until_char(buffer: &[u8], c: u8) -> usize {
    let mut i = 0;
    while buffer[i] != c {
        i += 1;
    }
    i
}

#[inline]
fn find_closing_bracket(buffer: &[u8], open: u8, close: u8) -> usize {
    let mut nested = 0;

    for (i, b) in buffer.iter().enumerate() {
        let b = *b;

        // Assumes there is at least one opening bracket before a closing one
        if b == open {
            nested += 1;
            // println!("Matched open: {} {}", b as char, open as char);
        } else if b == close {
            nested -= 1;

            // println!("Matched close: {} {}", b as char, close as char);
            if nested == 0 {
                return i;
            }
        }
    }

    panic!(
        "No closing bracket found in buffer! {}",
        from_utf8(buffer).unwrap()
    );
}
