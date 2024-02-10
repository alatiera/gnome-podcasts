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

use gtk::{gio, glib};

#[macro_use]
extern crate log;

#[macro_use]
extern crate html5ever;

// Exports the macros defined in utils to the namespace of the crate so they can be used
// easily without import
#[macro_use]
mod utils;

mod stacks;
mod widgets;

mod app;
#[rustfmt::skip]
mod config;
mod headerbar;
mod window;

mod manager;
mod settings;

mod episode_description_parser;
mod i18n;

use crate::app::PdApplication;

use once_cell::sync::Lazy;

pub static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().unwrap());

pub static MAINCONTEXT: Lazy<glib::MainContext> = Lazy::new(glib::MainContext::default);

#[cfg(test)]
fn init_gtk_tests() -> anyhow::Result<()> {
    gst::init()?;
    gtk::init()?;
    adw::init()?;
    register_resources()?;
    Ok(())
}

fn main() -> glib::ExitCode {
    pretty_env_logger::init();
    gst::init().expect("Error initializing gstreamer");
    gtk::init().expect("Error initializing gtk.");
    register_resources().expect("Error registering resources");

    PdApplication::run()
}

fn register_resources() -> anyhow::Result<()> {
    // Create Resource it will live as long the value lives.
    let gbytes = glib::Bytes::from_static(crate::config::RESOURCEFILE);
    let resource = gio::Resource::from_data(&gbytes)?;

    // Register the resource so it won't be dropped and will continue to live in
    // memory.
    gio::resources_register(&resource);

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
    HomeEpisode::default();
    EpisodeWidget::default();

    show_menu::ShowMenu::default();
    episode_menu::EpisodeMenu::default();

    Ok(())
}
