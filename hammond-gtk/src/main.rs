// extern crate glib;
extern crate gtk;
// extern crate gdk_pixbuf;

use gtk::prelude::*;
use gtk::Orientation;
use gtk::IconSize;
// use gtk::{CellRendererText, TreeStore, TreeView, TreeViewColumn};

use gtk::prelude::*;

fn create_child(name: &str) -> gtk::Box {
    let box_ = gtk::Box::new(Orientation::Vertical, 5);
    let img = gtk::Image::new_from_icon_name("gtk-missing-image", IconSize::Menu.into());
    let label = gtk::Label::new(name);
    box_.pack_start(&img, true, true, 0);
    box_.pack_start(&label, false, false, 0);
    box_
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    // Direct copy of the way gnome-music does albumview
    let glade_src = include_str!("../gtk/foo.ui");
    let builder = gtk::Builder::new_from_string(glade_src);

    // Get the main window
    let window: gtk::Window = builder.get_object("window1").unwrap();
    // Get the headerbar
    let header: gtk::HeaderBar = builder.get_object("headerbar1").unwrap();
    window.set_titlebar(&header);

    // Exit cleanly on delete event
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let flowbox: gtk::FlowBox = builder.get_object("flowbox1").unwrap();
    for _ in 0..10 {
        let f = create_child("placeholder");
        flowbox.add(&f);
    }


    window.show_all();
    gtk::main();
}
