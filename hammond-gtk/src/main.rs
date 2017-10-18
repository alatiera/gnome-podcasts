// extern crate glib;

extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate gtk;

extern crate diesel;
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

use widgets::podcast::*;
use views::podcasts_view::populate_podcasts_flowbox;

/*
THIS IS STILL A PROTOTYPE.
THE CODE IS TERIBLE AND USES UNWRAPS EVERYWHERE.
*/

fn build_ui() {
    let glade_src = include_str!("../gtk/foo.ui");
    let builder = gtk::Builder::new_from_string(glade_src);

    // Get the main window
    let window: gtk::Window = builder.get_object("window1").unwrap();
    // Get the Stack
    let stack: gtk::Stack = builder.get_object("stack1").unwrap();

    let db = Arc::new(Mutex::new(hammond_data::establish_connection()));
    let pd_widget = podcast_widget(db.clone(), None, None, None);
    stack.add_named(&pd_widget, "pdw");

    // FIXME:
    // GLib-GIO-WARNING **: Your application does not implement g_application_activate()
    // and has no handlers connected to the 'activate' signal.  It should do one of these.
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // Adapted copy of the way gnome-music does albumview
    // FIXME: flowbox childs activate with space/enter but not with clicks.
    let flowbox: gtk::FlowBox = builder.get_object("flowbox1").unwrap();
    let grid: gtk::Grid = builder.get_object("grid").unwrap();

    // Get the headerbar
    let header = headerbar::get_headerbar(db.clone(), stack.clone(), grid.clone());
    window.set_titlebar(&header);

    populate_podcasts_flowbox(db.clone(), stack.clone(), flowbox.clone());

    window.show_all();
    gtk::main();
}

// Copied from:
// https://github.com/GuillaumeGomez/process-viewer/blob/ \
// ddcb30d01631c0083710cf486caf04c831d38cb7/src/process_viewer.rs#L367
fn main() {
    loggerv::init_with_level(LogLevel::Info).unwrap();
    hammond_data::init().unwrap();

    // Not sure if needed.
    if gtk::init().is_err() {
        info!("Failed to initialize GTK.");
        return;
    }

    let application = gtk::Application::new(
        "com.gitlab.alatiera.Hammond",
        gio::ApplicationFlags::empty(),
    ).expect("Initialization failed...");

    application.connect_startup(move |_| {
        build_ui();
    });

    // Not sure if this will be kept.
    let original = ::std::env::args().collect::<Vec<_>>();
    let mut tmp = Vec::with_capacity(original.len());
    for i in 0..original.len() {
        tmp.push(original[i].as_str());
    }
    application.run(&tmp);
}
