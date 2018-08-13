use glib;
use gtk;
use gtk::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum UndoState {
    Shown,
    Hidden,
}

#[derive(Debug, Clone)]
pub(crate) struct InAppNotification {
    revealer: gtk::Revealer,
    text: gtk::Label,
    undo: gtk::Button,
    close: gtk::Button,
}

impl Default for InAppNotification {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/inapp_notif.ui");

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
    pub(crate) fn new<F, U>(text: &str, mut callback: F, undo_callback: Option<U>) -> Self
    where
        F: FnMut() -> glib::Continue + 'static,
        U: Fn() + 'static,
    {
        let notif = InAppNotification::default();
        notif.text.set_text(&text);

        let revealer = notif.revealer.clone();
        let id = timeout_add_seconds(6, move || {
            revealer.set_reveal_child(false);
            callback()
        });
        let id = Rc::new(RefCell::new(Some(id)));

        if undo_callback.is_some() {
            notif.set_undo_state(UndoState::Shown)
        };

        // Cancel the callback
        let revealer = notif.revealer.clone();
        notif.undo.connect_clicked(move |_| {
            let foo = id.borrow_mut().take();
            if let Some(id) = foo {
                glib::source::source_remove(id);
            }

            if let Some(ref f) = undo_callback {
                f();
            }

            // Hide the notification
            revealer.set_reveal_child(false);
        });

        // Hide the revealer when the close button is clicked
        let revealer = notif.revealer.clone();
        notif.close.connect_clicked(move |_| {
            revealer.set_reveal_child(false);
        });

        notif
    }

    // This is a separate method cause in order to get a nice animation
    // the revealer should be attached to something that displays it.
    // Previously we where doing it in the constructor, which had the result
    // of the animation being skipped cause there was no parent widget to display it.
    pub(crate) fn show(&self, overlay: &gtk::Overlay) {
        overlay.add_overlay(&self.revealer);
        // We need to display the notification after the widget is added to the overlay
        // so there will be a nice animation.
        self.revealer.set_reveal_child(true);
    }

    pub(crate) fn set_undo_state(&self, state: UndoState) {
        match state {
            UndoState::Shown => self.undo.show(),
            UndoState::Hidden => self.undo.hide(),
        }
    }
}
