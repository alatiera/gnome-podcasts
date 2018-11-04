// prefs.rs
//
// Copyright 2018 Measly Twerp <measlytwerp@gmail.com>
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


use gio;
use gio::{Settings, SettingsExt};
use gtk;
use gtk::prelude::*;

use i18n::i18n;

#[derive(Debug, Clone)]
pub(crate) struct Prefs {
    dialog: gtk::Window,
    dark_toggle: gtk::Switch,
    cleanup_value: gtk::SpinButton,
    cleanup_type: gtk::ComboBox,
}

impl Default for Prefs {
    fn default() -> Prefs {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/prefs.ui");

        let dialog = builder.get_object("prefs").unwrap();
        let dark_toggle = builder.get_object("dark_toggle").unwrap();
        let cleanup_value = builder.get_object("cleanup_value").unwrap();
        let cleanup_type = builder.get_object("cleanup_type").unwrap();

        Prefs {
            dialog,
            dark_toggle,
            cleanup_value,
            cleanup_type,
        }
    }
}

// TODO: Refactor components into smaller state machines
impl Prefs {
    pub(crate) fn new(settings: &Settings) -> Prefs {
        let h = Prefs::default();
        h.init(settings);
        h
    }

    pub(crate) fn init(&self, settings: &Settings) {
        settings.bind(
            "dark-theme",
            &self.dark_toggle,
            "active",
            gio::SettingsBindFlags::DEFAULT,
        );
        settings.bind(
            "cleanup-age-time",
            &self.cleanup_value,
            "value",
            gio::SettingsBindFlags::DEFAULT,
        );
        let cleanup_p = settings.get_string("cleanup-age-period").unwrap();
        let mut cleanup_pos = 0;
        let store = gtk::ListStore::new(&[gtk::Type::String]);
        for (i, item) in [
            i18n("Seconds"),
            i18n("Minutes"),
            i18n("Hours"),
            i18n("Days"),
            i18n("Weeks"),
        ]
        .iter()
        .enumerate()
        {
            let row: &[&dyn ToValue] = &[item];
            if item.to_lowercase() == cleanup_p {
                cleanup_pos = i as i32;
            }
            store.insert_with_values(None, &[0], &row);
        }
        self.cleanup_type.set_model(Some(&store));
        let renderer = gtk::CellRendererText::new();
        self.cleanup_type.pack_start(&renderer, true);
        self.cleanup_type.add_attribute(&renderer, "text", 0);
        self.cleanup_type.set_active(cleanup_pos);

        let weak_settings = settings.downgrade();
        let weak_store = store.downgrade();
        // Totally the prettiest stairway you ever saw
        self.cleanup_type.connect_changed(move |combo| {
            if let Some(ref treeiter) = combo.get_active_iter() {
                if let Some(store) = weak_store.upgrade() {
                    if let Some(s) = store.get_value(treeiter, 0).get::<&str>() {
                        if let Some(settings) = weak_settings.upgrade() {
                            settings.set_string("cleanup-age-period", &s.to_lowercase());
                        }
                    }
                }
            };
        });
    }

    pub(crate) fn show(&self, parent: &gtk::ApplicationWindow) {
        self.dialog.set_transient_for(Some(parent));
        self.dialog.set_modal(true);
        self.dialog.show_all();
    }
}
