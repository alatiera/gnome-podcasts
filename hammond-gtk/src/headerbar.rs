#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]

use gtk;
use gtk::prelude::*;

use diesel::prelude::SqliteConnection;
use index_feed;
use utils;

use std::sync::{Arc, Mutex};

pub fn get_headerbar(db: &Arc<Mutex<SqliteConnection>>, stack: &gtk::Stack) -> gtk::HeaderBar {
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
    let stack_clone = stack.clone();

    add_button.connect_clicked(move |_| {
        let tempdb = db_clone.lock().unwrap();
        let url = new_url.get_text().unwrap_or_default();
        // TODO: check if the feed is already present.
        let f = index_feed::insert_return_source(&tempdb, &url);
        drop(tempdb);
        info!("{:?} feed added", url);
        if let Ok(mut source) = f {
            // update the db
            utils::refresh_feed(&db_clone, &stack_clone, &mut source);
        } else {
            error!("Expected Error, feed probably already exists.");
            error!("Error: {:?}", f.unwrap_err());
        }

        // TODO: lock the button instead of hiding and add notification of feed added.
        // TODO: map the spinner
        add_popover_clone.hide();
    });
    add_popover.hide();
    add_toggle_button.set_popover(&add_popover);

    // TODO: make it a back arrow button, that will hide when appropriate,
    // and add a StackSwitcher when more views are added.
    let stack_clone = stack.clone();
    home_button.connect_clicked(move |_| {
        let grid = stack_clone.get_child_by_name("pd_grid").unwrap();
        stack_clone.set_visible_child(&grid)
    });

    let stack_clone = stack.clone();
    let db_clone = db.clone();
    // FIXME: There appears to be a memmory leak here.
    refresh_button.connect_clicked(move |_| {
        // fsdaa, The things I do for the borrow checker.
        utils::refresh_db(&db_clone, &stack_clone);
    });

    header
}
