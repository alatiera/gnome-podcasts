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

/// Takes a `window` and creates and attaches an `adw::AboutWindow` to it.
pub(crate) fn about_dialog(window: &gtk::ApplicationWindow) {
    // Feel free to add yourself if you contributed.
    // Please keep it sorted alphabetically
    let developers = vec![
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

    let designers = vec!["Tobias Bernard", "Sam Hewitt"];

    let dialog = adw::AboutWindow::builder()
        .application_icon(APP_ID)
        .comments(i18n("Podcast Client for the GNOME Desktop.").as_str())
        .copyright("© 2017-2021 Jordan Petridis")
        .license_type(gtk::License::Gpl30)
        .version(VERSION)
        .application_name(i18n("Podcasts"))
        .website("https://gitlab.gnome.org/World/podcasts")
        .issue_url("https://gitlab.gnome.org/World/podcasts/-/issues")
        .transient_for(window)
        .developer_name("Jordan Petridis, et al.")
        .developers(developers)
        .designers(designers)
        .translator_credits(i18n("translator-credits").as_str())
        .build();

    dialog.present();
}
