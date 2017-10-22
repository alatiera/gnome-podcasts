#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]

use glib;

use gtk;
// use gtk::prelude::*;

use hammond_data;
use hammond_data::index_feed::Feed;
use hammond_data::models::Source;
use diesel::prelude::SqliteConnection;

use std::thread;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver};

use views::podcasts_view;

// Create a thread local storage that will store the arguments to be transfered.
thread_local!(
    static GLOBAL: RefCell<Option<(Arc<Mutex<SqliteConnection>>,
    gtk::Stack,
    Receiver<bool>)>> = RefCell::new(None));

pub fn refresh_db(db: &Arc<Mutex<SqliteConnection>>, stack: &gtk::Stack) {
    // Create a async channel.
    let (sender, receiver) = channel();

    let db_clone = db.clone();
    // Pass the desired arguments into the Local Thread Storage.
    GLOBAL.with(move |global| {
        *global.borrow_mut() = Some((db_clone, stack.clone(), receiver));
    });

    // The implementation of how this is done is probably terrible but it works!.
    // TODO: add timeout option and error reporting.
    let db_clone = db.clone();
    thread::spawn(move || {
        let t = hammond_data::index_feed::index_loop(&db_clone, false);
        if t.is_err() {
            error!("Error While trying to update the database.");
            error!("Error msg: {}", t.unwrap_err());
        };
        sender.send(true).expect("Couldn't send data to channel");;

        // http://gtk-rs.org/docs/glib/source/fn.idle_add.html
        glib::idle_add(refresh_podcasts_view);
    });
}

pub fn refresh_feed(db: &Arc<Mutex<SqliteConnection>>, stack: &gtk::Stack, source: &mut Source) {
    let (sender, receiver) = channel();

    let db_clone = db.clone();
    GLOBAL.with(move |global| {
        *global.borrow_mut() = Some((db_clone, stack.clone(), receiver));
    });

    let db_clone = db.clone();
    let mut source_ = source.clone();
    // TODO: add timeout option and error reporting.
    thread::spawn(move || {
        let db_ = db_clone.clone();
        let db_ = db_.lock().unwrap();
        let foo_ = hammond_data::index_feed::refresh_source(&db_, &mut source_, false);
        drop(db_);

        if let Ok(x) = foo_ {
            let Feed(mut req, s) = x;
            let s = hammond_data::index_feed::complete_index_from_source(&mut req, &s, &db_clone);
            if s.is_err() {
                error!("Error While trying to update the database.");
                error!("Error msg: {}", s.unwrap_err());
            };

            sender.send(true).expect("Couldn't send data to channel");;
            glib::idle_add(refresh_podcasts_view);
        };
    });
}

fn refresh_podcasts_view() -> glib::Continue {
    GLOBAL.with(|global| {
        if let Some((ref db, ref stack, ref reciever)) = *global.borrow() {
            if reciever.try_recv().is_ok() {
                podcasts_view::update_podcasts_view(db, stack);
            }
        }
    });
    glib::Continue(false)
}
