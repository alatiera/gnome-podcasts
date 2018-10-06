use app::{APP_ID, VERSION};
use gtk;
use gtk::prelude::*;

use i18n::i18n;

// Totally copied it from fractal.
// https://gitlab.gnome.org/danigm/fractal/blob/503e311e22b9d7540089d735b92af8e8f93560c5/fractal-gtk/src/app.rs#L1883-1912
/// Given a `window` create and attach an `gtk::AboutDialog` to it.
pub(crate) fn about_dialog(window: &gtk::ApplicationWindow) {
    // Feel free to add yourself if you contributed.
    // Please keep it sorted alphabetically
    let authors = &[
        "Alexandre Franke",
        "Carlos Soriano",
        "Constantin Nickel",
        "Daniel García Moreno",
        "Felix Häcker",
        "Gabriele Musco",
        "Ivan Augusto",
        "James Wykeham-Martin",
        "Jordan Petridis",
        "Julian Sparber",
        "Matthew Martin",
        "Piotr Drąg",
        "Rowan Lewis",
        "Zander Brown",
    ];

    let dialog = gtk::AboutDialog::new();
    dialog.set_logo_icon_name(APP_ID);
    dialog.set_comments(i18n("Podcast Client for the GNOME Desktop.").as_str());
    dialog.set_copyright("© 2017, 2018 Jordan Petridis");
    dialog.set_license_type(gtk::License::Gpl30);
    dialog.set_modal(true);
    dialog.set_version(VERSION);
    dialog.set_program_name(&i18n("Podcasts"));
    dialog.set_website("https://wiki.gnome.org/Apps/Podcasts");
    dialog.set_website_label(i18n("Learn more about GNOME Podcasts").as_str());
    dialog.set_transient_for(window);

    dialog.set_artists(&["Tobias Bernard", "Sam Hewitt"]);
    dialog.set_authors(authors);
    dialog.set_translator_credits(i18n("translator-credits").as_str());

    dialog.connect_response(|dlg, _| dlg.destroy());

    dialog.show();
}
