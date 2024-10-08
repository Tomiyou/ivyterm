use std::{cell::RefCell, rc::Rc};

use glib::Propagation;
use gtk4::{gdk::ModifierType, Align, Box, EventControllerKey, GestureClick, Label, Orientation, ShortcutTrigger};
use libadwaita::{prelude::*, PreferencesGroup, PreferencesPage, PreferencesRow};

use crate::{application::IvyApplication, keyboard::Keybinding};

// TODO: Remove text from Keybinding (use to_label() instead)
struct LearningKeybinding {
    keybinding: Rc<RefCell<Keybinding>>,
    input_ctrl: EventControllerKey,
    display_widget: Label,
    row: PreferencesRow,
}

impl LearningKeybinding {
    pub fn new(
        keybinding: &Rc<RefCell<Keybinding>>,
        input_ctrl: &EventControllerKey,
        display_widget: &Label,
        row: &PreferencesRow,
    ) -> Self {
        Self {
            keybinding: keybinding.clone(),
            input_ctrl: input_ctrl.clone(),
            display_widget: display_widget.clone(),
            row: row.clone(),
        }
    }

    pub fn update_and_remove(&self, update: Option<String>) {
        if let Some(trigger) = update {
            let trigger = ShortcutTrigger::parse_string(&trigger);
            if let Some(trigger) = trigger {
                println!("Parsed trigger {}", trigger);
                // TODO: Store Keybindings in a global list and simply swap lists
            }
        }

        let binding = self.keybinding.borrow();
        let text = &binding.text;
        self.display_widget.set_label(text);

        // Remove keyboard controller if any
        println!("Removed controller");
        self.row.remove_controller(&self.input_ctrl);
    }
}

fn key_event_to_trigger(unicode: char, state: ModifierType) -> String {
    let mut ret = String::new();
    if state.contains(ModifierType::CONTROL_MASK) {
        ret.push_str("<Ctrl>");
    }
    if state.contains(ModifierType::SHIFT_MASK) {
        ret.push_str("<Shift>");
    }
    if state.contains(ModifierType::ALT_MASK) {
        ret.push_str("<Alt>");
    }
    ret.push(unicode);
    ret
}

fn create_keybinding_row(keybinding: Keybinding) -> PreferencesRow {
    let row_box = Box::new(Orientation::Horizontal, 0);

    let learning: Rc<RefCell<Option<LearningKeybinding>>> = Rc::new(RefCell::new(None));

    let label_widget = Label::builder()
        .label(keybinding.description)
        .halign(Align::Start)
        .hexpand(true)
        .build();
    row_box.append(&label_widget);

    let keybind_widget = Label::builder()
        .label(&keybinding.text)
        .halign(Align::End)
        .build();
    row_box.append(&keybind_widget);

    let keybinding = Rc::new(RefCell::new(keybinding));

    let row = PreferencesRow::builder()
        .child(&row_box)
        .css_classes(["setting_row"])
        .build();

    // Handle losing focus
    row.connect_has_focus_notify(glib::clone!(
        #[weak]
        learning,
        move |row| {
            if row.has_focus() == false {
                if let Some(learning) = learning.borrow_mut().take() {
                    learning.update_and_remove(None);
                }
            }
        }
    ));

    // Capture double click event
    let gesture_ctrl = GestureClick::new();
    gesture_ctrl.connect_released(move |gesture_ctrl, count, _, _| {
        // let keybinding = keybinding;
        // We are only interested in single clicks
        if count < 2 {
            return;
        }

        // If we are already learning there is no point continuing
        let mut binding = learning.borrow_mut();
        if binding.is_some() {
            return;
        }

        // Users just double-clicked this row, he wants to enter a new keybinding
        keybind_widget.set_label("Enter new keybinding...");

        // Start capturing keyboard
        let row: PreferencesRow = gesture_ctrl.widget().unwrap().downcast().unwrap();
        let keyboard_ctrl = EventControllerKey::new();

        // Start listening
        binding.replace(LearningKeybinding::new(
            &keybinding,
            &keyboard_ctrl,
            &keybind_widget,
            &row,
        ));

        keyboard_ctrl.connect_key_pressed(glib::clone!(
            #[weak]
            learning,
            #[upgrade_or]
            Propagation::Stop,
            move |_, keyval, keycode, state| {
                let unicode = keyval.to_unicode();
                println!("Controller input {:?} - {}", keyval.to_unicode(), keycode);
                if unicode.is_none() {
                    return Propagation::Stop;
                }
                let unicode = unicode.unwrap();

                let binding = learning.borrow();
                let _learning = binding
                    .as_ref()
                    .expect("Keybind learning input, but not learning!");
                match keycode {
                    9 => {
                        // Handle ESCAPE - ignore
                        _learning.update_and_remove(None);
                    }
                    22 => {
                        // Handle Backspace - unassign keybinding
                        _learning.update_and_remove(Some("".to_string()));
                    }
                    _ => {
                        let trigger = key_event_to_trigger(unicode, state);
                        _learning.update_and_remove(Some(trigger));
                    }
                }
                Propagation::Stop
            }
        ));

        row.add_controller(keyboard_ctrl);
        println!("Added controller");
    });
    row.add_controller(gesture_ctrl);

    row
}

pub fn create_keybinding_page(app: &IvyApplication) -> PreferencesPage {
    let group = PreferencesGroup::new();

    let keybindings = app.get_keybindings();
    for keybind in keybindings {
        let row = create_keybinding_row(keybind);
        group.add(&row);
    }

    let page = PreferencesPage::builder().title("Keybindings").build();
    page.add(&group);
    page
}
