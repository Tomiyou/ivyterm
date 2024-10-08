mod imp;

use glib::Object;
use libadwaita::glib;

glib::wrapper! {
    pub struct IvyLayout(ObjectSubclass<imp::IvyLayoutPriv>)
        @extends gtk4::LayoutManager;
}

impl IvyLayout {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
