#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]

// extern crate glib;

extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate gtk;

extern crate diesel;
extern crate dissolve;
extern crate hammond_data;
extern crate hammond_downloader;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate open;

use log::LogLevel;
use hammond_data::index_feed;

use std::sync::{Arc, Mutex};

use gtk::prelude::*;
use gio::ApplicationExt;

pub mod views;
pub mod widgets;
pub mod headerbar;
pub mod utils;

use views::podcasts_view;

/*
THIS IS STILL A PROTOTYPE.
*/

fn build_ui(app: &gtk::Application) {
    let db = Arc::new(Mutex::new(hammond_data::establish_connection()));

    // Get the main window
    let window = gtk::ApplicationWindow::new(app);
    window.set_default_size(1050, 600);
    app.add_window(&window);
    // Setup the Stack that will magane the switche between podcasts_view and podcast_widget.
    let stack = podcasts_view::setup_stack(&db);
    window.add(&stack);

    // FIXME:
    // GLib-GIO-WARNING **: Your application does not implement g_application_activate()
    // and has no handlers connected to the 'activate' signal.  It should do one of these.
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // Get the headerbar
    let header = headerbar::get_headerbar(&db, &stack);
    // Uncomment this when etag implementation is fixed and refesh_db thread is non blocking.
    // utils::refresh_db(&db, &stack);
    window.set_titlebar(&header);

    window.show_all();
    gtk::main();
}

// Copied from:
// https://github.com/GuillaumeGomez/process-viewer/blob/ \
// ddcb30d01631c0083710cf486caf04c831d38cb7/src/process_viewer.rs#L367
fn main() {
    loggerv::init_with_level(LogLevel::Info).unwrap();
    hammond_data::init().expect("Hammond Initialazation failed.");

    // Not sure if needed.
    if gtk::init().is_err() {
        info!("Failed to initialize GTK.");
        return;
    }

    let application = gtk::Application::new(
        "com.gitlab.alatiera.Hammond",
        gio::ApplicationFlags::empty(),
    ).expect("Initialization failed...");

    application.connect_startup(move |app| {
        build_ui(app);
    });

    // Not sure if this will be kept.
    let original = ::std::env::args().collect::<Vec<_>>();
    let mut tmp = Vec::with_capacity(original.len());
    for i in 0..original.len() {
        tmp.push(original[i].as_str());
    }
    application.run(&tmp);
}
