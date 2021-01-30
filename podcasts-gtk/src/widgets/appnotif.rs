// appnotif.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use glib::clone;
use gtk::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum State {
    Shown,
    Hidden,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum SpinnerState {
    Active,
    Stopped,
}

#[derive(Debug, Clone)]
pub(crate) struct InAppNotification {
    revealer: gtk::Revealer,
    text: gtk::Label,
    undo: gtk::Button,
    close: gtk::Button,
    spinner: gtk::Spinner,
}

impl Default for InAppNotification {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/inapp_notif.ui");

        let revealer: gtk::Revealer = builder.get_object("revealer").unwrap();
        let text: gtk::Label = builder.get_object("text").unwrap();
        let undo: gtk::Button = builder.get_object("undo").unwrap();
        let close: gtk::Button = builder.get_object("close").unwrap();
        let spinner = builder.get_object("spinner").unwrap();

        InAppNotification {
            revealer,
            text,
            undo,
            close,
            spinner,
        }
    }
}

impl InAppNotification {
    /// Creates a new instance of InAppNotification
    ///
    /// # Arguments
    ///
    /// * `text` - Text which is displayed within the revealer
    /// * `timer` - Time in ms until the callback is called
    /// * `callback` - Function to call after `timer` is passed.
    ///                You will probably want to call `set_reveal_child(false)` within it
    /// * `undo_callback` - If undo_callback is `is_some()`, then the revealer will include an undo-button.
    ///                     If the undo-button is pressed, undo_callback will be called.
    pub(crate) fn new<F, U>(
        text: &str,
        timer: u32,
        mut callback: F,
        undo_callback: Option<U>,
    ) -> Self
    where
        F: FnMut(gtk::Revealer) -> glib::Continue + 'static,
        U: Fn() + 'static,
    {
        let notif = InAppNotification::default();
        notif.text.set_text(&text);

        let mut time = 0;
        let interval = 250;
        let id = timeout_add(
            interval,
            clone!(@weak notif.revealer as revealer => @default-return glib::Continue(false), move || {
                    if time < timer {
                        time += interval;
                        return glib::Continue(true);
                    };
                    callback(revealer)
            }),
        );
        let id = Rc::new(RefCell::new(Some(id)));

        if undo_callback.is_some() {
            notif.set_undo_state(State::Shown)
        };

        notif
            .undo
            .connect_clicked(clone!(@weak notif.revealer as revealer => move |_| {
                if let Some(id) = id.borrow_mut().take() {
                    // Cancel the callback
                    glib::source::source_remove(id);
                }

                if let Some(ref f) = undo_callback {
                    f();
                }

                // Hide the notification
                revealer.set_reveal_child(false);
            }));

        // Hide the revealer when the close button is clicked
        notif
            .close
            .connect_clicked(clone!(@weak notif.revealer as revealer => move |_| {
                revealer.set_reveal_child(false);
            }));

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

    pub(crate) fn set_undo_state(&self, state: State) {
        match state {
            State::Shown => self.undo.show(),
            State::Hidden => self.undo.hide(),
        }
    }

    pub(crate) fn set_close_state(&self, state: State) {
        match state {
            State::Shown => self.close.show(),
            State::Hidden => self.close.hide(),
        }
    }

    pub(crate) fn set_spinner_state(&self, state: SpinnerState) {
        match state {
            SpinnerState::Active => {
                self.spinner.start();
                self.spinner.show();
            }
            SpinnerState::Stopped => {
                self.spinner.stop();
                self.spinner.hide();
            }
        }
    }

    pub(crate) unsafe fn destroy(self) {
        self.revealer.destroy();
    }
}
