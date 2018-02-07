#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr, needless_pass_by_value))]
// #![deny(unused_extern_crates, unused)]

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

extern crate chrono;
extern crate dissolve;
extern crate hammond_data;
extern crate hammond_downloader;
extern crate humansize;
extern crate loggerv;
extern crate open;
extern crate send_cell;
extern crate url;
// extern crate rayon;

// use rayon::prelude::*;
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

pub mod utils;
pub mod manager;
pub mod static_resource;

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

    // This set's the app to dark mode.
    // It wiil be in the user's preference later.
    // Uncomment it to run with the dark theme variant.
    // let settings = gtk::Settings::get_default().unwrap();
    // settings.set_property_gtk_application_prefer_dark_theme(true);

    App::new().run();
}
