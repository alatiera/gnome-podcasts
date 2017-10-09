// extern crate glib;
extern crate gtk;
// extern crate gdk_pixbuf;

// use gtk::prelude::*;
// use gtk::{CellRendererText, TreeStore, TreeView, TreeViewColumn};

use gtk::prelude::*;

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let glade_src = include_str!("../gtk/foo.ui");
    let builder = gtk::Builder::new_from_string(glade_src);

    // Get the main window
    let window :gtk::Window = builder.get_object("appwindow1").unwrap();
    // Get the headerbar
    let header :gtk::HeaderBar = builder.get_object("headerbar1").unwrap();
    window.set_titlebar(&header);

    // Exit cleanly on delete event
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    window.show_all();
    gtk::main();
}
