use glib;
use gtk;
use gtk::prelude::*;

use app::Action;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct InAppNotification {
    pub overlay: gtk::Overlay,
    revealer: gtk::Revealer,
    text: gtk::Label,
    undo: gtk::Button,
}

impl Default for InAppNotification {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/inapp_notif.ui");

        let overlay: gtk::Overlay = builder.get_object("notif_overlay").unwrap();
        let revealer: gtk::Revealer = builder.get_object("notif_revealer").unwrap();
        let text: gtk::Label = builder.get_object("notif_label").unwrap();
        let undo: gtk::Button = builder.get_object("undo_button").unwrap();

        InAppNotification {
            overlay,
            revealer,
            text,
            undo,
        }
    }
}

impl InAppNotification {
    pub fn new<F>(text: String, callback: F, sender: Sender<Action>) -> Self
    where
        F: FnMut() -> glib::Continue + 'static,
    {
        let notif = InAppNotification::default();

        notif.text.set_text(&text);
        notif.revealer.set_reveal_child(true);

        let id = timeout_add_seconds(60, callback);
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

        notif
    }
}
