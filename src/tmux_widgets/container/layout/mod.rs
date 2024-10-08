mod imp;

use glib::Object;
use libadwaita::{glib, prelude::*};

use super::{separator::TmuxSeparator, TmuxContainer};

glib::wrapper! {
    pub struct TmuxLayout(ObjectSubclass<imp::TmuxLayoutPriv>)
        @extends gtk4::LayoutManager;
}

impl TmuxLayout {
    pub fn new() -> Self {
        Object::builder().build()
    }
}

fn drag_update(separator: &TmuxSeparator, container: &TmuxContainer, x: f64, y: f64) {}
