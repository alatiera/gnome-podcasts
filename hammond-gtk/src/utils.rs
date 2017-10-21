#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]

// use glib;

use gtk;
// use gtk::prelude::*;

use hammond_data;
use hammond_data::index_feed::Feed;
use hammond_data::models::Source;
use diesel::prelude::SqliteConnection;

use std::thread;
use std::sync::{Arc, Mutex};

use views::podcasts_view;

pub fn refresh_db(db: &Arc<Mutex<SqliteConnection>>, stack: &gtk::Stack) {
    let db_clone = db.clone();
    let handle = thread::spawn(move || {
        let t = hammond_data::index_feed::index_loop(&db_clone, false);
        if t.is_err() {
            error!("Error While trying to update the database.");
            error!("Error msg: {}", t.unwrap_err());
        };
    });
    // FIXME: atm freezing the ui till update is done.
    // Make it instead emmit a signal on update completion.
    // TODO: emit a signal in order to update the podcast widget.
    let _ = handle.join();

    podcasts_view::update_podcasts_view(db, stack);
}

pub fn refresh_feed(db: &Arc<Mutex<SqliteConnection>>, stack: &gtk::Stack, source: &mut Source) {
    let db_clone = db.clone();
    let mut source_ = source.clone();
    // TODO: add timeout option and error reporting.
    let handle = thread::spawn(move || {
        let db_ = db_clone.lock().unwrap();
        let foo_ = hammond_data::index_feed::refresh_source(&db_, &mut source_, false);
        drop(db_);

        if let Ok(x) = foo_ {
            let Feed(mut req, s) = x;
            let s = hammond_data::index_feed::complete_index_from_source(&mut req, &s, &db_clone);
            if s.is_err() {
                error!("Error While trying to update the database.");
                error!("Error msg: {}", s.unwrap_err());
            };
        };
    });
    // FIXME: atm freezing the ui till update is done.
    // Make it instead emmit a signal on update completion.
    // TODO: emit a signal in order to update the podcast widget.
    let _ = handle.join();

    podcasts_view::update_podcasts_view(db, stack);
}

// https://github.
// com/needle-and-thread/vocal/blob/8b21f1c18c2be32921e84e289576a659ab3c8f2e/src/Utils/Utils.
// vala#L136
// TODO:
// pub fn html_to_markup(s: String) -> String {
//     let markup = glib::uri_escape_string(s.as_str(), None, true);

//     let markup = if let Some(m) = markup {
//         m
//     } else {
//         warn!("unable to unescape markup: {}", s);
//         s
//     };
//     // let markup = s;


//     info!("{}", markup);
//     markup
// }
