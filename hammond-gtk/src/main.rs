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
use diesel::prelude::*;
use hammond_data::index_feed;
use hammond_downloader::downloader;

use std::thread;
use std::sync::{Arc, Mutex};

use gtk::prelude::*;
use gio::ApplicationExt;
use gdk_pixbuf::Pixbuf;

pub mod views;
pub mod widgets;

use widgets::podcast::{create_flowbox_child, podcast_liststore, podcast_widget};

/*
THIS IS STILL A PROTOTYPE.
THE CODE IS TERIBLE, SPAGHETTI AND HAS UNWRAPS EVERYWHERE.
*/
fn refresh_db(db: Arc<Mutex<SqliteConnection>>) {
    let db_clone = db.clone();
    thread::spawn(move || {
        hammond_data::index_feed::index_loop(db_clone.clone(), false).unwrap();
    });
}

// I am sorry about the spaghetti code.
// Gonna clean it up when the GUI is a bit usuable.
fn build_ui() {
    let glade_src = include_str!("../gtk/foo.ui");
    let header_src = include_str!("../gtk/headerbar.ui");
    let builder = gtk::Builder::new_from_string(glade_src);
    let header_build = gtk::Builder::new_from_string(header_src);

    // Get the main window
    let window: gtk::Window = builder.get_object("window1").unwrap();
    // Get the Stack
    let stack: gtk::Stack = builder.get_object("stack1").unwrap();

    let db = Arc::new(Mutex::new(hammond_data::establish_connection()));
    let pd_widget = podcast_widget(db.clone(), None, None, None);
    stack.add_named(&pd_widget, "pdw");
    // Get the headerbar
    let header: gtk::HeaderBar = header_build.get_object("headerbar1").unwrap();
    window.set_titlebar(&header);

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

    // Stolen from gnome-news:
    // https://github.com/GNOME/gnome-news/blob/master/data/ui/headerbar.ui
    let add_toggle_button: gtk::MenuButton = header_build.get_object("add-toggle-button").unwrap();
    let add_popover: gtk::Popover = header_build.get_object("add-popover").unwrap();
    let new_url: gtk::Entry = header_build.get_object("new-url").unwrap();
    let add_button: gtk::Button = header_build.get_object("add-button").unwrap();
    // TODO: check if url exists in the db and lock the button
    new_url.connect_changed(move |url| {
        println!("{:?}", url.get_text());
    });
    let add_popover_clone = add_popover.clone();
    let db_clone = db.clone();
    add_button.connect_clicked(move |_| {
        let tempdb = db_clone.lock().unwrap();
        let url = new_url.get_text().unwrap();
        let _ = index_feed::insert_return_source(&tempdb, &url);
        drop(tempdb);
        println!("{:?} feed added", url);

        // update the db
        refresh_db(db_clone.clone());

        // TODO: lock the button instead of hiding and add notification of feed added.
        add_popover_clone.hide();
    });
    add_popover.hide();
    add_toggle_button.set_popover(&add_popover);

    let _search_button: gtk::Button = header_build.get_object("searchbutton").unwrap();

    // TODO: make it a back arrow button, that will hide when appropriate,
    // and add a StackSwitcher when more views are added.
    let home_button: gtk::Button = header_build.get_object("homebutton").unwrap();
    let grid_clone = grid.clone();
    let stack_clone = stack.clone();
    home_button.connect_clicked(move |_| stack_clone.set_visible_child(&grid_clone));

    let refresh_button: gtk::Button = header_build.get_object("refbutton").unwrap();
    // FIXME: There appears to be a memmory leak here.
    let db_clone = db.clone();
    refresh_button.connect_clicked(move |_| {
        // fsdaa, The things I do for the borrow checker.
        refresh_db(db_clone.clone());
    });

    let tempdb = db.lock().unwrap();
    let pd_model = podcast_liststore(&tempdb);
    drop(tempdb);

    // Get a ListStore iterator at the first element.
    let iter = pd_model.get_iter_first().unwrap();

    // Iterate the podcast view.
    loop {
        let title = pd_model.get_value(&iter, 1).get::<String>().unwrap();
        let description = pd_model.get_value(&iter, 2).get::<String>().unwrap();
        let image_uri = pd_model.get_value(&iter, 4).get::<String>();

        let imgpath = downloader::cache_image(&title, image_uri.as_ref().map(|s| s.as_str()));

        let pixbuf = if let Some(i) = imgpath {
            Pixbuf::new_from_file_at_scale(&i, 200, 200, true).ok()
        } else {
            None
        };

        let f = create_flowbox_child(&title, pixbuf.clone());
        let stack_clone = stack.clone();
        let db_clone = db.clone();
        f.connect_activate(move |_| {
            let pdw = stack_clone.get_child_by_name("pdw").unwrap();
            stack_clone.remove(&pdw);
            let pdw = podcast_widget(
                db_clone.clone(),
                Some(title.as_str()),
                Some(description.as_str()),
                pixbuf.clone(),
            );
            stack_clone.add_named(&pdw, "pdw");
            stack_clone.set_visible_child(&pdw);
            println!("Hello World!, child activated");
        });
        flowbox.add(&f);

        if !pd_model.iter_next(&iter) {
            break;
        }
    }

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
