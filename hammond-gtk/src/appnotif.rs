use glib;
use gtk;
use gtk::prelude::*;

use app::Action;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct InAppNotification {
    pub revealer: gtk::Revealer,
    text: gtk::Label,
    undo: gtk::Button,
    close: gtk::Button,
}

impl Default for InAppNotification {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/inapp_notif.ui");

        let revealer: gtk::Revealer = builder.get_object("revealer").unwrap();
        let text: gtk::Label = builder.get_object("text").unwrap();
        let undo: gtk::Button = builder.get_object("undo").unwrap();
        let close: gtk::Button = builder.get_object("close").unwrap();

        InAppNotification {
            revealer,
            text,
            undo,
            close,
        }
    }
}

impl InAppNotification {
    pub fn new<F>(text: String, mut callback: F, sender: Sender<Action>) -> Self
    where
        F: FnMut() -> glib::Continue + 'static,
    {
        let notif = InAppNotification::default();
        notif.text.set_text(&text);

        let revealer = notif.revealer.clone();
        let id = timeout_add_seconds(6, move || {
            revealer.set_reveal_child(false);
            callback()
        });
        let id = Rc::new(RefCell::new(Some(id)));

        // Cancel the callback
        let revealer = notif.revealer.clone();
        notif.undo.connect_clicked(move |_| {
            let foo = id.borrow_mut().take();
            if let Some(id) = foo {
                glib::source::source_remove(id);
            }

            // Hide the notification
            revealer.set_reveal_child(false);
            // Refresh the widget if visible
            if let Err(err) = sender.send(Action::RefreshWidgetIfVis) {
                error!(
                    "Something went horribly wrong with the Action channel: {}",
                    err
                )
            }
        });

        // Hide the revealer when the close button is clicked
        let revealer = notif.revealer.clone();
        notif.close.connect_clicked(move |_| {
            revealer.set_reveal_child(false);
        });

        notif
    }

    // This is a seperate method cause in order to get a nice animation
    // the revealer should be attached to something that will display it.
    // Previouslyi we where doing it in the constructor, which had the result
    // of the animation being skipped cause there was no parent widget to display it.
    pub fn show(&self) {
        self.revealer.set_reveal_child(true);
    }
}
