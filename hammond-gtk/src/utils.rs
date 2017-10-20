// use glib;

use hammond_data;
use diesel::prelude::SqliteConnection;

use std::thread;
use std::sync::{Arc, Mutex};

pub fn refresh_db(db: Arc<Mutex<SqliteConnection>>) {
    let db_clone = db.clone();
    thread::spawn(move || {
        let t = hammond_data::index_feed::index_loop(db_clone.clone(), false);
        if t.is_err() {
            error!("Error While trying to update the database.");
            error!("Error msg: {}", t.unwrap_err());
        };
    });
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
