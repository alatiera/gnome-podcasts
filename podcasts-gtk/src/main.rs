// main.rs
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

// Enable lint group collections
#![warn(nonstandard_style, rust_2018_idioms, bad_style, unused)]
// standalone lints
#![warn(
    const_err,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    unconditional_recursion,
    while_true,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    elided_lifetimes_in_paths,
    missing_copy_implementations
)]
#![allow(clippy::type_complexity)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[macro_use]
extern crate html5ever;

use gtk::prelude::*;

// Exports the macros defined in utils to the namespace of the crate so they can be used
// easily without import
#[macro_use]
mod utils;

mod stacks;
mod widgets;

mod app;
mod config;
mod headerbar;
mod window;

mod manager;
mod settings;

mod episode_description_parser;
mod i18n;

use crate::app::PdApplication;

#[cfg(test)]
fn init_gtk_tests() -> anyhow::Result<()> {
    // if gtk::is_initialized() {
    //     assert!(gtk::is_initialized_main_thread())
    // } else {
    //     gtk::init()?;
    //     register_resources()?;
    // }

    gst::init()?;
    gtk::init()?;
    libhandy::init();
    register_resources()?;
    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    gst::init().expect("Error initializing gstreamer");
    gtk::init().expect("Error initializing gtk.");
    libhandy::init();
    register_resources().expect("Error registering resources");

    // Add custom style
    let provider = gtk::CssProvider::new();
    gtk::CssProvider::load_from_resource(&provider, "/org/gnome/Podcasts/gtk/style.css");
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("Error initializing gtk css provider."),
        &provider,
        600,
    );

    PdApplication::run();
}

fn register_resources() -> anyhow::Result<()> {
    let res = gio::Resource::load(config::PKGDATADIR.to_owned() + "/resources.gresource")?;
    gio::resources_register(&res);
    Ok(())
}

#[test]
// Even while running the tests with -j 1 and --test-threads=1,
// cargo seems to create new threads and gtk refuses to initialize again.
// So we run every gtk related test here.
fn test_stuff() -> anyhow::Result<()> {
    use crate::headerbar::Header;
    use crate::widgets::*;

    init_gtk_tests()?;

    // If a widget does not exist in the `GtkBuilder`(.ui) file this should panic and fail.
    Header::default();
    ShowsView::default();
    ShowWidget::default();
    HomeView::default();
    HomeEpisode::default();
    EpisodeWidget::default();
    EmptyView::default();
    EmptyShow::default();
    EpisodeDescription::default();

    appnotif::InAppNotification::default();
    show_menu::ShowMenu::default();
    episode_menu::EpisodeMenu::default();

    Ok(())
}
