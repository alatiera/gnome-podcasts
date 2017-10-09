// extern crate glib;
extern crate gtk;
// extern crate gdk_pixbuf;
extern crate hammond_data;

use gtk::prelude::*;
use gtk::Orientation;
use gtk::IconSize;
// use gtk::{CellRendererText, TreeStore, TreeView, TreeViewColumn};

use hammond_data::dbqueries;

use gtk::prelude::*;

// TODO: setup a img downloader, caching system, and then display them.
fn create_child(name: &str) -> gtk::Box {
    let box_ = gtk::Box::new(Orientation::Vertical, 5);
    let img = gtk::Image::new_from_icon_name("gtk-missing-image", IconSize::Menu.into());
    let label = gtk::Label::new(name);
    box_.set_size_request(200, 200);
    box_.pack_start(&img, true, true, 0);
    box_.pack_start(&label, false, false, 0);
    box_
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    // Adapted copy of the way gnome-music does albumview
    let glade_src = include_str!("../gtk/foo.ui");
    let builder = gtk::Builder::new_from_string(glade_src);

    // Get the main window
    let window: gtk::Window = builder.get_object("window1").unwrap();
    // Get the headerbar
    let header: gtk::HeaderBar = builder.get_object("headerbar1").unwrap();
    window.set_titlebar(&header);

    let refresh_button : gtk::Button = builder.get_object("refbutton").unwrap();
    // TODO: Have a small dropdown menu
    let add_button : gtk::Button = builder.get_object("addbutton").unwrap();
    let search_button : gtk::Button = builder.get_object("searchbutton").unwrap();
    let home_button : gtk::Button = builder.get_object("homebutton").unwrap();

    // FIXME: This locks the ui atm.
    refresh_button.connect_clicked(|_| {
        let db = hammond_data::establish_connection();
        hammond_data::index_feed::index_loop(db, false).unwrap();
    });

    // Exit cleanly on delete event
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let flowbox: gtk::FlowBox = builder.get_object("flowbox1").unwrap();

    // TODO: This should be in a TreeStore.
    let db = hammond_data::establish_connection();
    let podcasts = dbqueries::get_podcasts(&db).unwrap();

    for pd in &podcasts {
        let f = create_child(pd.title());
        flowbox.add(&f);
    }

    window.show_all();
    gtk::main();
}
