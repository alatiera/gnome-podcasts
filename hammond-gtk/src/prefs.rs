#![allow(dead_code)]

use gio;
use gio::{Settings, SettingsExt};
use gtk;
use gtk::prelude::*;

#[derive(Debug, Clone)]
pub struct Prefs {
    dialog: gtk::Window,
    dark_toggle: gtk::Switch,
    startup_toggle: gtk::Switch,
    auto_toggle: gtk::Switch,
    refresh_value: gtk::SpinButton,
    refresh_type: gtk::ComboBox,
    cleanup_value: gtk::SpinButton,
    cleanup_type: gtk::ComboBox,
}

impl Default for Prefs {
    fn default() -> Prefs {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/prefs.ui");

        let dialog = builder.get_object("prefs").unwrap();
        let dark_toggle = builder.get_object("dark_toggle").unwrap();
        let startup_toggle = builder.get_object("startup_toggle").unwrap();
        let auto_toggle = builder.get_object("auto_toggle").unwrap();
        let refresh_value = builder.get_object("refresh_value").unwrap();
        let refresh_type = builder.get_object("refresh_type").unwrap();
        let cleanup_value = builder.get_object("cleanup_value").unwrap();
        let cleanup_type = builder.get_object("cleanup_type").unwrap();

        Prefs {
            dialog,
            dark_toggle,
            startup_toggle,
            auto_toggle,
            refresh_value,
            refresh_type,
            cleanup_value,
            cleanup_type,
        }
    }
}

// TODO: Refactor components into smaller state machines
impl Prefs {
    pub fn new(settings: &Settings) -> Prefs {
        let h = Prefs::default();
        h.init(settings);
        h
    }

    pub fn init(&self, settings: &Settings) {
        settings.bind(
            "dark-theme",
            &self.dark_toggle,
            "active",
            gio::SettingsBindFlags::DEFAULT,
        );
        settings.bind(
            "refresh-on-startup",
            &self.startup_toggle,
            "active",
            gio::SettingsBindFlags::DEFAULT,
        );
        settings.bind(
            "refresh-interval",
            &self.auto_toggle,
            "active",
            gio::SettingsBindFlags::DEFAULT,
        );
        settings.bind(
            "refresh-interval-time",
            &self.refresh_value,
            "value",
            gio::SettingsBindFlags::DEFAULT,
        );
        settings.bind(
            "cleanup-age-time",
            &self.cleanup_value,
            "value",
            gio::SettingsBindFlags::DEFAULT,
        );
        let refresh_p = settings.get_string("refresh-interval-period").unwrap();
        let mut refresh_pos = 0;
        let cleanup_p = settings.get_string("cleanup-age-period").unwrap();
        let mut cleanup_pos = 0;
        let store = gtk::ListStore::new(&[gtk::Type::String]);
        for (i, item) in ["Seconds", "Minutes", "Hours", "Days", "Weeks"].iter().enumerate() {
            let row: &[&ToValue] = &[item];
            if item.to_lowercase() == refresh_p {
                refresh_pos = i;
            }
            if item.to_lowercase() == cleanup_p {
                cleanup_pos = i;
            }
            store.insert_with_values(None, &[0], &row);
        }
        for combo in &[self.refresh_type.clone(), self.cleanup_type.clone()] {
            combo.set_model(Some(&store));
            let renderer = gtk::CellRendererText::new();
            combo.pack_start(&renderer, true);
            combo.add_attribute(&renderer, "text", 0);
        }
        self.refresh_type.set_active(refresh_pos);
        self.refresh_type
            .connect_changed(clone!(settings, store => move |combo| {
            let value = store.get_value(&combo.get_active_iter().unwrap(), 0);
            let value: &str = value.get().unwrap();
            settings.set_string("refresh-interval-period", &value.to_lowercase());
        }));
        self.cleanup_type.set_active(cleanup_pos);
        self.cleanup_type
            .connect_changed(clone!(settings, store => move |combo| {
            let value = store.get_value(&combo.get_active_iter().unwrap(), 0);
            let value: &str = value.get().unwrap();
            settings.set_string("cleanup-age-period", &value.to_lowercase());
        }));
    }

    pub fn show(&self, parent: &gtk::ApplicationWindow) {
        self.dialog.set_transient_for(Some(parent));
        self.dialog.set_modal(true);
        self.dialog.show_all();
    }
}
