use gio;
use gtk;
use gtk::prelude::*;

#[derive(Debug, Clone)]
// TODO: split this into smaller
pub struct Prefs {
    dialog: gtk::Window,
    refresh_type: gtk::ComboBox,
    cleanup_type: gtk::ComboBox,
}

impl Default for Prefs {
    fn default() -> Prefs {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/prefs.ui");

        let dialog = builder.get_object("prefs").unwrap();
        let refresh_type = builder.get_object("refresh_type").unwrap();
        let cleanup_type = builder.get_object("cleanup_type").unwrap();

        Prefs {
            dialog,
            refresh_type,
            cleanup_type,
        }
    }
}

// TODO: Refactor components into smaller state machines
impl Prefs {
    pub fn new(
        settings: &gio::Settings,
    ) -> Prefs {
        let h = Prefs::default();
        h.init(settings);
        h
    }

    pub fn init(
        &self,
        _settings: &gio::Settings,
    ) {
        println!("TODO");
        let store = gtk::ListStore::new(&[gtk::Type::String]);
        for item in ["Seconds", "Minutes", "Hours", "Days", "Weeks"].iter() {
            let row = [&(item) as &ToValue];
            store.insert_with_values(None, &[0], &row);
        }
        for combo in [self.refresh_type.clone(), self.cleanup_type.clone()].iter() {
            combo.set_model(Some(&store));
            let renderer = gtk::CellRendererText::new();
            combo.pack_start (&renderer, true);
            combo.add_attribute (&renderer, "text", 0);
        }
        //settings.get_string("cleanup-age-period").unwrap();
    }

    pub fn show (&self, parent: &gtk::Window) {
        self.dialog.set_transient_for(Some(parent));
        self.dialog.set_modal(true);
        self.dialog.show_all();
    }
}
