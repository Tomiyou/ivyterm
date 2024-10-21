use std::str::from_utf8;

use enumflags2::{BitFlag, BitFlags};
use gtk4::Orientation;
use log::debug;

use super::{LayoutFlags, LayoutSync, Rectangle, TmuxPane};

pub fn parse_tmux_layout(buffer: &[u8]) -> LayoutSync {
    // Example layout:
    // @0 a705,80x31,0,0[80x15,0,0,0,80x15,0,16{40x15,0,16,1,39x15,41,16,2}] a85f,80x31,0,0,2
    debug!("Given layout {}", from_utf8(buffer).unwrap());

    // Skip initial @, if it exists
    let buffer = if buffer[0] == b'@' {
        debug!("Skipping @");
        &buffer[1..]
    } else {
        &buffer
    };

    // Read tab ID
    let (tab_id, bytes_read) = read_first_u32(buffer);
    let buffer = &buffer[bytes_read + 1..];

    // Parse real layout
    let space_position = read_until_char(buffer, b' ');
    let real_hierarchy = parse_layout_root(&buffer[..space_position]);
    let buffer = &buffer[space_position + 1..];

    // Parse visible layout
    let space_position = read_until_char(buffer, b' ');
    let visible_hierarchy = parse_layout_root(&buffer[..space_position]);
    let buffer = &buffer[space_position + 1..];

    // Parse window flags
    debug!("Flags {}", from_utf8(buffer).unwrap());
    let flags = parse_flags(buffer);

    LayoutSync {
        tab_id,
        layout: real_hierarchy,
        visible_layout: visible_hierarchy,
        flags,
    }
}

#[inline]
fn parse_flags(buffer: &[u8]) -> BitFlags<LayoutFlags> {
    let mut flags = LayoutFlags::empty();

    for byte in buffer {
        match *byte {
            b'*' => flags |= LayoutFlags::HasFocus,
            b'Z' => flags |= LayoutFlags::IsZoomed,
            _ => {},
        }
    }

    flags
}

#[inline]
fn parse_layout_root(buffer: &[u8]) -> Vec<TmuxPane> {
    debug!("parse_layout_root layout {}", from_utf8(buffer).unwrap());

    // Skip the initial whatever
    let bytes_read = read_until_char(buffer, b',');
    let buffer = &buffer[bytes_read + 1..];

    let mut hierarchy = Vec::new();
    parse_layout_recursive(buffer, &mut hierarchy, 0);

    hierarchy
}

fn parse_layout_recursive(buffer: &[u8], hierarchy: &mut Vec<TmuxPane>, nested: u32) {
    // We can assume that layout is purse ASCII text
    let mut buffer = buffer;

    loop {
        // print_tab(nested);
        debug!("Remaining buffer: {}", from_utf8(buffer).unwrap());
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
            hierarchy.push(TmuxPane::Terminal(pane_id, allocation));

            buffer = &buffer[bytes_read..];
        } else {
            // This is a Container
            let (orientation, open, close) = if buffer[0] == '[' as u8 {
                (Orientation::Vertical, b'[', b']')
            } else {
                (Orientation::Horizontal, b'{', b'}')
            };
            hierarchy.push(TmuxPane::Container(orientation, allocation));

            // recursively call parse_tmux_layout
            let bytes_read = find_closing_bracket(buffer, open, close);
            parse_layout_recursive(&buffer[1..bytes_read], hierarchy, nested + 1);
            hierarchy.push(TmuxPane::Return);

            buffer = &buffer[bytes_read + 1..];
        }

        if buffer.is_empty() {
            break;
        }

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
    while i < buffer.len() && buffer[i] != c {
        i += 1;
    }
    i
}

#[inline]
pub fn find_closing_bracket(buffer: &[u8], open: u8, close: u8) -> usize {
    let mut nested = 0;

    for (i, b) in buffer.iter().enumerate() {
        let b = *b;

        // Assumes there is at least one opening bracket before a closing one
        if b == open {
            nested += 1;
        } else if b == close {
            nested -= 1;

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
