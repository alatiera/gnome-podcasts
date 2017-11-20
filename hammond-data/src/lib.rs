#![recursion_limit = "1024"]
#![cfg_attr(feature = "cargo-clippy", allow(blacklisted_name))]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

extern crate chrono;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rayon;
extern crate reqwest;
extern crate rfc822_sanitizer;
extern crate rss;
extern crate url;
extern crate xdg;

pub mod dbqueries;
pub mod utils;
pub mod models;
pub mod feed;
pub mod errors;
mod parser;
mod schema;

// use r2d2_diesel::ConnectionManager;
// use diesel::SqliteConnection;
use diesel::prelude::*;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
// use std::time::Duration;

// type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
type Database = Arc<Mutex<SqliteConnection>>;

lazy_static!{
    #[allow(dead_code)]
    static ref HAMMOND_XDG: xdg::BaseDirectories = {
        xdg::BaseDirectories::with_prefix("hammond").unwrap()
    };

    static ref _HAMMOND_DATA: PathBuf = {
        HAMMOND_XDG.create_data_directory(HAMMOND_XDG.get_data_home()).unwrap()
    };

    static ref _HAMMOND_CONFIG: PathBuf = {
        HAMMOND_XDG.create_config_directory(HAMMOND_XDG.get_config_home()).unwrap()
    };

    pub static ref HAMMOND_CACHE: PathBuf = {
        HAMMOND_XDG.create_cache_directory(HAMMOND_XDG.get_cache_home()).unwrap()
    };

    pub static ref DL_DIR: PathBuf = {
        HAMMOND_XDG.create_data_directory("Downloads").unwrap()
    };

    // static ref POOL: Pool = init_pool(DB_PATH.to_str().unwrap());

    static ref DB: Arc<Mutex<SqliteConnection>> = Arc::new(Mutex::new(establish_connection()));
}

#[cfg(not(test))]
lazy_static! {
    static ref DB_PATH: PathBuf = HAMMOND_XDG.place_data_file("hammond.db").unwrap();
}

#[cfg(test)]
extern crate tempdir;

#[cfg(test)]
lazy_static! {
    static ref TEMPDIR: tempdir::TempDir = {
        tempdir::TempDir::new("hammond_unit_test").unwrap()
    };

    static ref DB_PATH: PathBuf = TEMPDIR.path().join("hammond.db");
}

pub fn connection() -> Database {
    // POOL.clone()
    Arc::clone(&DB)
}

// fn init_pool(db_path: &str) -> Pool {
//     let config = r2d2::Config::builder()
//         // .pool_size(60)
//         // .min_idle(Some(60))
//         // .connection_timeout(Duration::from_secs(60))
//         .build();
//     let manager = ConnectionManager::<SqliteConnection>::new(db_path);
//     let pool = r2d2::Pool::new(config, manager).expect("Failed to create pool.");
//     info!("Database pool initialized.");

//     {
//         let db = pool.clone().get().unwrap();
//         utils::run_migration_on(&*db).unwrap();
//     }

//     pool
// }

pub fn establish_connection() -> SqliteConnection {
    let database_url = DB_PATH.to_str().unwrap();
    let db = SqliteConnection::establish(database_url)
        .expect(&format!("Error connecting to {}", database_url));
    utils::run_migration_on(&db).unwrap();
    db
}
