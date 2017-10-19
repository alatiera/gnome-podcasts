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
