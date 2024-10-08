// list-windows -F "#{window_layout}"
// @0 6a18,191x47,0,0[191x23,0,0,0,191x23,0,24{95x23,0,24,1,95x23,96,24,2}]

pub fn parse_tmux_layout(buffer: &[u8]) -> usize {
    // We can assume that layout is purse ASCII text
    let mut total_bytes_read = 0;

    // Read width
    let (width, bytes_read) = read_first_u32(&buffer[total_bytes_read..]);
    total_bytes_read += bytes_read;

    // Read height
    let (height, bytes_read) = read_first_u32(&buffer[total_bytes_read..]);
    total_bytes_read += bytes_read;

    // Read x coordinate
    let (x, bytes_read) = read_first_u32(&buffer[total_bytes_read..]);
    total_bytes_read += bytes_read;

    // Read y coordinate
    let (y, bytes_read) = read_first_u32(&buffer[total_bytes_read..]);
    total_bytes_read += bytes_read;

    // let buffer = &buffer[bytes_read..];

    // Now we have to determine if this is a Pane or a Container
    if buffer[0] == ',' as u8 {
        // This is a Pane
    } else {
        // This is a Container
        // recursively call parse_tmux_layout
    }

    println!("Parsed layout: {}x{} | {},{}", width, height, x, y);
    total_bytes_read
}

#[inline]
pub fn read_first_u32(buffer: &[u8]) -> (u32, usize) {
    let mut i = 0;
    let mut number: u32 = 0;

    // Read buffer char by char (assuming ASCII) and parse number
    while buffer[i] > 47 && buffer[i] < 58 {
        number *= 10;
        number += (buffer[i] - 48) as u32;
        i += 1;
    }
    (number, i + 1)
}
