mod imp;

use glib::{Object, Propagation};
use gtk4::{
    gdk::ModifierType, glib, Align, Box, EventControllerKey, GestureClick, Label, Orientation,
    ShortcutTrigger,
};
use libadwaita::{prelude::*, subclass::prelude::*, PreferencesGroup, PreferencesRow};

use crate::{application::IvyApplication, keyboard::Keybinding};

glib::wrapper! {
    pub struct KeybindingPage(ObjectSubclass<imp::KeybindingPage>)
        @extends libadwaita::PreferencesPage, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl KeybindingPage {
    pub fn new(app: &IvyApplication) -> Self {
        let page: Self = Object::builder().build();
        page.set_title("Keybindings");

        // Create a row for each keybinding
        let group = PreferencesGroup::new();
        let keybindings = app.get_keybindings();
        let keybindings: Vec<(Keybinding, Label)> = keybindings
            .into_iter()
            .enumerate()
            .map(|(idx, keybinding)| {
                let (row, label) = create_keybinding_row(idx, &keybinding, &page);
                group.add(&row);
                (keybinding, label)
            })
            .collect();

        // Listen to keyboard events whenever user wants to assign a keybinding
        let keyboard_ctrl = EventControllerKey::new();
        keyboard_ctrl.connect_key_pressed(glib::clone!(
            #[weak]
            page,
            #[upgrade_or]
            Propagation::Stop,
            move |_, keyval, keycode, state| {
                let unicode = keyval.to_unicode();
                if unicode.is_none() {
                    return Propagation::Stop;
                }
                let unicode = unicode.unwrap();

                // Check for special keys such as ESCAPE and BACKSPACE
                match keycode {
                    9 => {
                        // Handle ESCAPE - ignore
                        page.user_keyboard_input(None);
                    }
                    22 => {
                        // Handle Backspace - unassign keybinding
                        page.user_keyboard_input(Some(""));
                    }
                    _ => {
                        let trigger = key_event_to_trigger(unicode, state);
                        if let Some(trigger) = trigger {
                            page.user_keyboard_input(Some(&trigger));
                        } else {
                            // If user pressed something we don't want, ignore it
                            page.user_keyboard_input(None);
                        }
                    }
                }

                Propagation::Stop
            }
        ));

        let imp = page.imp();
        imp.keyboard_ctrl.replace(Some(keyboard_ctrl));
        imp.keybindings.replace(keybindings);

        page.add(&group);
        page
    }

    fn row_listening_changed(&self, new_listener: usize, listen: bool) {
        let imp = self.imp();
        let old_listener = imp.listening.get();

        // If listen is false, it means we stop listening in any case
        if listen == false {
            if let Some(old_listener) = old_listener {
                // Stop listening
                imp.enable_keyboard(false);
                imp.update_label_text(old_listener, false);
                imp.listening.replace(None);
            }

            return;
        }

        // Check if we are already listening to a row
        if let Some(old_listener) = old_listener {
            // If we are already listening to this row, we are done
            if old_listener == new_listener {
                return;
            }

            // Listening focus has changed, reset the name
            imp.update_label_text(old_listener, false);
        }

        // Start listening
        imp.listening.replace(Some(new_listener));
        imp.enable_keyboard(true);
        // Show to user that we are listening for new keybinding
        imp.update_label_text(new_listener, true);
    }

    fn user_keyboard_input(&self, new_keybinding: Option<&str>) {
        let imp = self.imp();

        // Check if we are listening
        let listener = match imp.listening.take() {
            Some(listener) => listener,
            None => {
                // This should not happen!!
                panic!("Keybinding user input detected, but we are not listening for one!");
            }
        };

        if let Some(new_keybinding) = new_keybinding {
            let mut keybindings = imp.keybindings.borrow_mut();

            // Wrap it in {} so 'keybinding' borrow is dropped
            let new_trigger = ShortcutTrigger::parse_string(new_keybinding);
            let action = {
                let (keybinding, _) = keybindings.get_mut(listener).unwrap();
                keybinding.trigger = new_trigger.clone();
                keybinding.action
            };

            // Walk through keybindings and check for keybinding collisions
            if let Some(trigger) = new_trigger {
                for (keybinding, label) in keybindings.iter_mut() {
                    // Don't unbind self
                    if keybinding.action == action {
                        continue;
                    }

                    // If new trigger collides with an existing one, unbind the existing one
                    if let Some(collision) = &keybinding.trigger {
                        if trigger.equal(collision) {
                            keybinding.trigger = None;
                            set_text_from_trigger(label, &keybinding.trigger);
                        }
                    }
                }
            }
        }

        // Stop listening
        imp.enable_keyboard(false);

        // Update displayed Keybinding
        imp.update_label_text(listener, false);
    }

    pub fn get_keybindings(&self) -> Vec<Keybinding> {
        let keybindings = self.imp().keybindings.borrow();
        keybindings
            .iter()
            .map(|(keybinding, _)| keybinding.clone())
            .collect()
    }
}

fn create_keybinding_row(
    idx: usize,
    keybinding: &Keybinding,
    page: &KeybindingPage,
) -> (PreferencesRow, Label) {
    let row_box = Box::new(Orientation::Horizontal, 0);

    let description = Label::builder()
        .label(keybinding.description)
        .halign(Align::Start)
        .hexpand(true)
        .build();
    row_box.append(&description);

    let accelerator_label = Label::builder().halign(Align::End).build();
    set_text_from_trigger(&accelerator_label, &keybinding.trigger);
    row_box.append(&accelerator_label);

    let row = PreferencesRow::builder()
        .child(&row_box)
        .css_classes(["setting_row"])
        .build();

    // Handle losing focus
    row.connect_has_focus_notify(glib::clone!(
        #[weak]
        page,
        move |row| {
            // Stop listening if row loses focus
            if row.has_focus() == false {
                page.row_listening_changed(idx, false);
            }
        }
    ));

    let gesture_ctrl = GestureClick::new();
    gesture_ctrl.connect_released(glib::clone!(
        #[weak]
        page,
        move |_, count, _, _| {
            if count < 2 {
                return;
            }

            // Row was double clicked, start listening for keyboard input
            page.row_listening_changed(idx, true);
        }
    ));
    row.add_controller(gesture_ctrl);

    (row, accelerator_label)
}

#[inline]
fn set_text_from_trigger(label: &Label, trigger: &Option<ShortcutTrigger>) {
    if let Some(trigger) = trigger {
        let text = trigger.to_str();
        label.set_label(text.as_str());
    } else {
        label.set_label("");
    }
}

#[inline]
fn key_event_to_trigger(unicode: char, state: ModifierType) -> Option<String> {
    let mut ret = String::new();
    let mut modifier_count = 0;
    if state.contains(ModifierType::CONTROL_MASK) {
        ret.push_str("<Ctrl>");
        modifier_count += 1;
    }
    if state.contains(ModifierType::SHIFT_MASK) {
        ret.push_str("<Shift>");
        modifier_count += 1;
    }
    if state.contains(ModifierType::ALT_MASK) {
        ret.push_str("<Alt>");
        modifier_count += 1;
    }

    // There should be at least 1 modifier, so we don't get keybindings such as a single
    // character
    if modifier_count < 1 {
        return None;
    }

    ret.push(unicode);
    Some(ret)
}
