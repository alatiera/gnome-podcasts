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
use hammond_data::utils::checkup;

use gtk::prelude::*;
use gio::ApplicationExt;
use std::rc::Rc;

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

mod utils;
mod static_resource;

fn build_ui(app: &gtk::Application) {
    // Get the main window
    let window = gtk::ApplicationWindow::new(app);
    window.set_default_size(860, 640);

    // Get the headerbar
    let header = Rc::new(headerbar::Header::default());
    let ct = content::Content::new(header.clone());
    header.init(ct.clone());
    window.set_titlebar(&header.container);
    window.add(&ct.get_stack());

    window.connect_delete_event(|w, _| {
        w.destroy();
        Inhibit(false)
    });

    // Update on startup
    gtk::timeout_add_seconds(
        30,
        clone!(ct => move || {
        utils::refresh_feed(ct.clone(), None);
        glib::Continue(false)
    }),
    );
    // Auto-updater, runs every hour.
    // TODO: expose the interval in which it run to a user setting.
    // TODO: show notifications.
    gtk::timeout_add_seconds(
        3600,
        clone!(ct => move || {
        utils::refresh_feed(ct.clone(), None);
        glib::Continue(true)
    }),
    );

    gtk::idle_add(move || {
        let _ = checkup();
        glib::Continue(false)
    });

    window.show_all();
    window.activate();
    app.connect_activate(move |_| ());
}

fn main() {
    use gio::ApplicationExtManual;

    // TODO: make the the logger a cli -vv option
    loggerv::init_with_level(LogLevel::Info).unwrap();
    static_resource::init().expect("Something went wrong with the resource file initialization.");

    let application = gtk::Application::new("org.gnome.Hammond", gio::ApplicationFlags::empty())
        .expect("Initialization failed...");

    // Add custom style
    let provider = gtk::CssProvider::new();
    gtk::CssProvider::load_from_resource(&provider, "/org/gnome/hammond/gtk/style.css");
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::get_default().unwrap(),
        &provider,
        600,
    );

    application.connect_startup(move |app| {
        build_ui(app);
    });

    // application.run(&[]);
    ApplicationExtManual::run(&application, &[]);
}
