// aboutdialog.rs
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

use crate::config::{APP_ID, VERSION};
use gtk::prelude::*;

use crate::i18n::i18n;

/// Takes a `window` and creates and attaches an `gtk::AboutDialog` to it.
pub(crate) fn about_dialog(window: &gtk::ApplicationWindow) {
    // Feel free to add yourself if you contributed.
    // Please keep it sorted alphabetically
    let authors = [
        "Alexandre Franke",
        "Carlos Soriano",
        "Constantin Nickel",
        "Daniel García Moreno",
        "Felix Häcker",
        "Gabriele Musco",
        "Ivan Augusto",
        "James Wykeham-Martin",
        "Jordan Petridis",
        "Jordan Williams",
        "Julian Hofer",
        "Julian Sparber",
        "Matthew Martin",
        "Piotr Drąg",
        "Rowan Lewis",
        "Zander Brown",
    ];

    let dialog = gtk::AboutDialog::new();
    dialog.set_logo_icon_name(Some(APP_ID));
    dialog.set_comments(Some(i18n("Podcast Client for the GNOME Desktop.").as_str()));
    dialog.set_copyright(Some("© 2017, 2018 Jordan Petridis"));
    dialog.set_license_type(gtk::License::Gpl30);
    dialog.set_modal(true);
    dialog.set_version(Some(VERSION));
    dialog.set_program_name(Some(&i18n("Podcasts")));
    dialog.set_website(Some("https://wiki.gnome.org/Apps/Podcasts"));
    dialog.set_website_label(&i18n("Learn more about GNOME Podcasts"));
    dialog.set_transient_for(Some(window));

    dialog.set_artists(&["Tobias Bernard", "Sam Hewitt"]);
    dialog.set_authors(&authors);
    dialog.set_translator_credits(Some(i18n("translator-credits").as_str()));

    dialog.show();
}
