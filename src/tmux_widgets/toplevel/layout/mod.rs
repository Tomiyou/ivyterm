mod imp;

use glib::Object;
use libadwaita::glib;

glib::wrapper! {
    pub struct TopLevelLayout(ObjectSubclass<imp::TopLevelLayoutPriv>)
        @extends gtk4::LayoutManager;
}

impl TopLevelLayout {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
