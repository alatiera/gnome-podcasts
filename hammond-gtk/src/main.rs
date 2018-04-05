#![cfg_attr(feature = "cargo-clippy",
            allow(clone_on_ref_ptr, needless_pass_by_value, useless_format, blacklisted_name,
                  match_same_arms))]
#![allow(unknown_lints)]
#![deny(unused_extern_crates, unused)]

extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate glib;
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
extern crate html2pango;
extern crate humansize;
extern crate loggerv;
extern crate open;
extern crate rayon;
extern crate regex;
extern crate reqwest;
extern crate send_cell;
extern crate serde_json;
extern crate take_mut;
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

// They do not need to be public
// But it helps when looking at the generated docs.
pub mod views;
pub mod widgets;
pub mod stacks;

pub mod headerbar;
pub mod app;

pub mod settings;
pub mod utils;
pub mod manager;
pub mod static_resource;
pub mod appnotif;

use app::App;

fn main() {
    // TODO: make the the logger a cli -vv option
    loggerv::init_with_level(Level::Info).expect("Error initializing loggerv.");
    gtk::init().expect("Error initializing gtk.");
    static_resource::init().expect("Something went wrong with the resource file initialization.");

    // Add custom style
    let provider = gtk::CssProvider::new();
    gtk::CssProvider::load_from_resource(&provider, "/org/gnome/hammond/gtk/style.css");
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
        &provider,
        600,
    );

    App::new().run();
}
