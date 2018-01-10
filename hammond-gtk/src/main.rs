#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr, needless_pass_by_value))]

extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate glib;
extern crate gtk;

extern crate chrono;
extern crate diesel;
extern crate dissolve;
extern crate hammond_data;
extern crate hammond_downloader;
extern crate humansize;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate open;
extern crate regex;
extern crate send_cell;
// extern crate rayon;

// use rayon::prelude::*;
use log::LogLevel;

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

mod views;
mod widgets;
mod headerbar;
mod content;
mod app;

mod utils;
mod manager;
mod static_resource;

use app::App;

fn main() {
    // TODO: make the the logger a cli -vv option
    loggerv::init_with_level(LogLevel::Info).unwrap();
    gtk::init().expect("Error initializing gtk");
    static_resource::init().expect("Something went wrong with the resource file initialization.");

    // Add custom style
    let provider = gtk::CssProvider::new();
    gtk::CssProvider::load_from_resource(&provider, "/org/gnome/hammond/gtk/style.css");
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::get_default().unwrap(),
        &provider,
        600,
    );

    // This set's the app to dark mode.
    // It wiil be in the user's preference later but for now
    // I will abuse my power and force it on everyone till then :P.
    let settings = gtk::Settings::get_default().unwrap();
    settings.set_property_gtk_application_prefer_dark_theme(true);

    App::new().run();
}
