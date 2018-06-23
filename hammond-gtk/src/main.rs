#![cfg_attr(
    feature = "cargo-clippy",
    allow(clone_on_ref_ptr, blacklisted_name, match_same_arms, option_map_unit_fn)
)]
#![allow(unknown_lints)]
// Enable lint group collections
#![warn(nonstandard_style, edition_2018, rust_2018_idioms, bad_style, unused)]
// standalone lints
#![warn(
    const_err, improper_ctypes, non_shorthand_field_patterns, no_mangle_generic_items,
    overflowing_literals, plugin_as_library, private_no_mangle_fns, private_no_mangle_statics,
    unconditional_recursion, unions_with_drop_fields, while_true, missing_debug_implementations,
    trivial_casts, trivial_numeric_casts, elided_lifetime_in_paths, missing_copy_implementations
)]
#![deny(warnings)]

extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate glib;
extern crate gstreamer as gst;
extern crate gstreamer_player as gst_player;
extern crate gtk;

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

extern crate chrono;
extern crate crossbeam_channel;
extern crate hammond_data;
extern crate hammond_downloader;
extern crate html2text;
extern crate humansize;
extern crate loggerv;
extern crate open;
extern crate rayon;
extern crate regex;
extern crate reqwest;
extern crate send_cell;
extern crate serde_json;
extern crate url;

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
mod headerbar;

mod manager;
mod settings;
mod static_resource;
mod utils;

use app::App;

fn main() {
    // TODO: make the the logger a cli -vv option
    loggerv::init_with_level(Level::Info).expect("Error initializing loggerv.");
    gtk::init().expect("Error initializing gtk.");
    gst::init().expect("Error initializing gstreamer");
    static_resource::init().expect("Something went wrong with the resource file initialization.");

    // Add custom style
    let provider = gtk::CssProvider::new();
    gtk::CssProvider::load_from_resource(&provider, "/org/gnome/Hammond/gtk/style.css");
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
        &provider,
        600,
    );

    App::new().run();
}
