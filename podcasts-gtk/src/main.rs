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

#![cfg_attr(
    feature = "cargo-clippy",
    allow(clone_on_ref_ptr, blacklisted_name, match_same_arms,)
)]
#![allow(unknown_lints)]
// Enable lint group collections
#![warn(nonstandard_style, edition_2018, rust_2018_idioms, bad_style, unused)]
// standalone lints
#![warn(
    const_err,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    plugin_as_library,
    unconditional_recursion,
    unions_with_drop_fields,
    while_true,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    elided_lifetime_in_paths,
    missing_copy_implementations
)]

#[macro_use]
extern crate failure;
// #[macro_use]
// extern crate failure_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use log::Level;

use gtk::prelude::*;

// http://gtk-rs.org/tuto/closures
#[macro_export]
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

mod stacks;
mod widgets;

mod app;
mod config;
mod headerbar;
mod window;

mod manager;
mod settings;
mod static_resource;
mod utils;

mod i18n;

use crate::app::PdApplication;

#[cfg(test)]
fn init_gtk_tests() -> Result<(), failure::Error> {
    // if gtk::is_initialized() {
    //     assert!(gtk::is_initialized_main_thread())
    // } else {
    //     gtk::init()?;
    //     static_resource::init()?;
    // }

    gtk::init()?;
    static_resource::init()?;
    gst::init()?;
    Ok(())
}

fn main() {
    // TODO: make the the logger a cli -vv option
    loggerv::init_with_level(Level::Info).expect("Error initializing loggerv.");
    gtk::init().expect("Error initializing gtk.");
    gst::init().expect("Error initializing gstreamer");
    static_resource::init().expect("Something went wrong with the resource file initialization.");

    // Add custom style
    let provider = gtk::CssProvider::new();
    gtk::CssProvider::load_from_resource(&provider, "/org/gnome/Podcasts/gtk/style.css");
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
        &provider,
        600,
    );

    PdApplication::run();
}

#[test]
// Even while running the tests with -j 1 and --test-threads=1,
// cargo seems to create new threads and gtk refuses to initialize again.
// So we run every gtk related test here.
fn test_stuff() -> Result<(), failure::Error> {
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

    appnotif::InAppNotification::default();
    show_menu::ShowMenu::default();

    Ok(())
}
