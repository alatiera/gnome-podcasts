use hammond_data;
use diesel::prelude::*;

use std::thread;
use std::sync::{Arc, Mutex};

pub fn refresh_db(db: Arc<Mutex<SqliteConnection>>) {
    let db_clone = db.clone();
    thread::spawn(move || {
        // FIXME: Handle unwrap
        hammond_data::index_feed::index_loop(db_clone.clone(), false).unwrap();
    });
}
