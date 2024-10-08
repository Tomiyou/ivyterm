mod imp;

use glib::{subclass::types::ObjectSubclassIsExt, Object};
use libadwaita::{glib, prelude::*, Application, ApplicationWindow};

use crate::tmux::Tmux;

glib::wrapper! {
    pub struct IvyWindow(ObjectSubclass<imp::IvyWindowPriv>)
        @extends ApplicationWindow, gtk4::Window;
        // @extends gtk::Button, gtk::Widget;
}

impl IvyWindow {
    pub fn new(app: &Application, title: &str, default_width: i32, default_height: i32) -> Self {
        let window: Self = Object::builder().build();
        window.set_application(Some(app));
        window.set_title(Some(title));
        window.set_default_width(default_width);
        window.set_default_height(default_height);

        println!("Created new window!");

        window
    }

    pub fn init_tmux(&self, tmux: Tmux) {
        self.imp().tmux.replace(Some(tmux));
    }
}
