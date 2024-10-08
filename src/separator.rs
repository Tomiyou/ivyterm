use gtk4::{Orientation, Separator};
use libadwaita::Bin;

pub fn new_separator(orientation: Orientation) -> Bin {
    let (separator_orientation, css_class, cursor) = match orientation {
        Orientation::Horizontal => (
            Orientation::Vertical,
            "separator_cont_vertical",
            "col-resize",
        ),
        Orientation::Vertical => (
            Orientation::Horizontal,
            "separator_cont_horizontal",
            "row-resize",
        ),
        _ => panic!("Unable to invert orientation to create separator"),
    };

    // Create separator widget
    let separator = Separator::new(separator_orientation);
    let separator_container = Bin::builder()
        .child(&separator)
        .css_classes(vec![css_class])
        .build();

    separator_container
}
