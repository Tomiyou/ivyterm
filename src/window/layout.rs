use std::str::from_utf8;

use gtk4::Orientation;
use libadwaita::prelude::*;

use crate::{
    container::Container, terminal::Terminal, toplevel::TopLevel,
    window::IvyWindow,
};

pub fn parse_tmux_layout(buffer: &[u8], window: &IvyWindow) {
    // Read tab ID
    let (tab_id, bytes_read) = read_first_u32(buffer);
    let buffer = &buffer[bytes_read + 1..];

    // Skip the initial whatever
    let bytes_read = read_until_char(buffer, b',');
    let buffer = &buffer[bytes_read + 1..];

    // Either get the existing tab, or spawn a new one if it does not exist yet
    let top_level = if let Some(top_level) = window.get_top_level(tab_id) {
        top_level
    } else {
        window.new_tab(Some(tab_id))
    };

    // Parse the recursive layout
    println!(
        "Tab id is {}, remaining buffer: {}",
        tab_id,
        from_utf8(buffer).unwrap()
    );
    parse_layout_recursive(buffer, window, &top_level, None, 0);
}

fn parse_layout_recursive(
    buffer: &[u8],
    window: &IvyWindow,
    top_level: &TopLevel,
    parent: Option<&Container>,
    nested: u32,
) {
    // We can assume that layout is purse ASCII text
    let mut buffer = buffer;
    let mut is_first_child = true;

    fn print_tab(nested: u32) {
        for i in 0..nested {
            print!("    ");
        }
    }

    // print_tab(nested);
    // println!("parse_layout_recursive: {}", from_utf8(buffer).unwrap());

    loop {
        // print_tab(nested);
        // println!("Remaining buffer: |{}|", from_utf8(buffer).unwrap());

        // TODO: What if already exists
        if !is_first_child {
            if let Some(parent) = parent {
                todo!();
                // let orientation = parent.orientation();
                // let new_separator = new_separator(orientation);
                // parent.append(&new_separator);
            }
        }

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

        // Now we have to determine if this is a Pane or a Container
        if buffer[0] == ',' as u8 {
            // This is a Pane
            buffer = &buffer[1..];

            let (pane_id, bytes_read) = read_first_u32(buffer);
            buffer = &buffer[bytes_read..];

            if let Some(pane) = window.get_pane(pane_id) {
                // Pane exists already
            } else {
                let new_terminal = Terminal::new(top_level, window, Some(pane_id));

                if let Some(parent) = parent {
                    parent.append(&new_terminal);
                } else {
                    println!("Pane without Container parent");
                    top_level.set_child(Some(&new_terminal));
                }

                top_level.register_terminal(&new_terminal);
            }
        } else {
            // This is a Container
            let (orientation, open, close) = if buffer[0] == '[' as u8 {
                (Orientation::Vertical, b'[', b']')
            } else {
                (Orientation::Horizontal, b'{', b'}')
            };

            // TODO: What if already exists
            let new_container = Container::new(orientation, 0);
            if let Some(parent) = parent {
                parent.append(&new_container);
            } else {
                top_level.set_child(Some(&new_container));
            }

            // recursively call parse_tmux_layout
            let bytes_read = find_closing_bracket(buffer, open, close);
            parse_layout_recursive(
                &buffer[1..bytes_read],
                window,
                top_level,
                Some(&new_container),
                nested + 1,
            );

            buffer = &buffer[bytes_read + 1..];
        }

        if buffer.is_empty() {
            break;
        }

        is_first_child = false;
        buffer = &buffer[1..];
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
