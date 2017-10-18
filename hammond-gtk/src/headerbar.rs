use gtk;
use gtk::prelude::*;

use diesel::prelude::*;
use index_feed;
use utils;

use std::sync::{Arc, Mutex};

pub fn get_headerbar(
    db: Arc<Mutex<SqliteConnection>>,
    stack: gtk::Stack,
    grid: gtk::Grid,
) -> gtk::HeaderBar {
    let builder = include_str!("../gtk/headerbar.ui");
    let builder = gtk::Builder::new_from_string(builder);

    let header: gtk::HeaderBar = builder.get_object("headerbar1").unwrap();
    let home_button: gtk::Button = builder.get_object("homebutton").unwrap();
    let refresh_button: gtk::Button = builder.get_object("refbutton").unwrap();
    let _search_button: gtk::Button = builder.get_object("searchbutton").unwrap();

    let add_toggle_button: gtk::MenuButton = builder.get_object("add-toggle-button").unwrap();
    let add_popover: gtk::Popover = builder.get_object("add-popover").unwrap();
    let new_url: gtk::Entry = builder.get_object("new-url").unwrap();
    let add_button: gtk::Button = builder.get_object("add-button").unwrap();
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
        utils::refresh_db(db_clone.clone());

        // TODO: lock the button instead of hiding and add notification of feed added.
        add_popover_clone.hide();
    });
    add_popover.hide();
    add_toggle_button.set_popover(&add_popover);

    // TODO: make it a back arrow button, that will hide when appropriate,
    // and add a StackSwitcher when more views are added.
    home_button.connect_clicked(move |_| stack.set_visible_child(&grid));

    // FIXME: There appears to be a memmory leak here.
    refresh_button.connect_clicked(move |_| {
        // fsdaa, The things I do for the borrow checker.
        utils::refresh_db(db.clone());
    });

    header
}
