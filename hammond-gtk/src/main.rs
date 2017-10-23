extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate glib;
extern crate gtk;

// extern crate diesel;
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

mod views;
mod widgets;
mod headerbar;

mod utils;

use views::podcasts_view;

/*
THIS IS STILL A PROTOTYPE.
*/

fn build_ui(app: &gtk::Application) {
    let db = Arc::new(Mutex::new(hammond_data::establish_connection()));

    // Get the main window
    let window = gtk::ApplicationWindow::new(app);
    window.set_default_size(1050, 600);
    // Setup the Stack that will magane the switche between podcasts_view and podcast_widget.
    let stack = podcasts_view::setup_stack(&db);
    window.add(&stack);

    window.connect_delete_event(|w, _| {
        w.destroy();
        Inhibit(false)
    });

    // Get the headerbar
    let header = headerbar::get_headerbar(&db, &stack);

    // TODO: add delay, cause else theres lock contention for the db obj.
    // utils::refresh_db(db.clone(), stack.clone());
    window.set_titlebar(&header);

    window.show_all();
    window.activate();
    app.connect_activate(move |_| ());
}

fn main() {
    // TODO: make the the logger a cli -vv option
    loggerv::init_with_level(LogLevel::Info).unwrap();
    hammond_data::init().expect("Hammond Initialazation failed.");

    let application = gtk::Application::new(
        "com.gitlab.alatiera.Hammond",
        gio::ApplicationFlags::empty(),
    ).expect("Initialization failed...");

    application.connect_startup(move |app| {
        build_ui(app);
    });

    application.run(&[]);
}
