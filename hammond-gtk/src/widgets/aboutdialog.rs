use gtk;
use gtk::prelude::*;

// Totally copied it from fractal.
// https://gitlab.gnome.org/danigm/fractal/blob/503e311e22b9d7540089d735b92af8e8f93560c5/fractal-gtk/src/app.rs#L1883-1912
/// Given a `window` create and attach an `gtk::AboutDialog` to it.
pub fn about_dialog(window: &gtk::ApplicationWindow) {
    // Feel free to add yourself if you contribured.
    let authors = &[
        "Carlos Soriano",
        "Constantin Nickel",
        "Gabriele Musco",
        "Ivan Augusto",
        "James Wykeham-Martin",
        "Jordan Petridis",
        "Julian Sparber",
        "Rowan Lewis",
        "Zander Brown",
    ];

    let dialog = gtk::AboutDialog::new();
    // Waiting for a logo.
    dialog.set_logo_icon_name("org.gnome.Hammond");
    dialog.set_comments("Podcast Client for the GNOME Desktop.");
    dialog.set_copyright("© 2017, 2018 Jordan Petridis");
    dialog.set_license_type(gtk::License::Gpl30);
    dialog.set_modal(true);
    // TODO: make it show it fetches the commit hash from which it was built
    // and the version number is kept in sync automaticly
    dialog.set_version("0.4.0");
    dialog.set_program_name("Hammond");
    // TODO: Need a wiki page first.
    // dialog.set_website("https://wiki.gnome.org/Design/Apps/Potential/Podcasts");
    // dialog.set_website_label("Learn more about Hammond");
    dialog.set_transient_for(window);

    dialog.set_artists(&["Tobias Bernard", "Sam Hewitt"]);
    dialog.set_authors(authors);

    dialog.connect_response(|dlg, _| dlg.destroy());

    dialog.show();
}
