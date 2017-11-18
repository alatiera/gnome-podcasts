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
extern crate rayon;
extern crate reqwest;
extern crate rfc822_sanitizer;
extern crate rss;
extern crate xdg;

pub mod dbqueries;
pub mod utils;
pub mod models;
pub mod feed;
pub mod errors;
mod parser;
mod schema;

use diesel::migrations::RunMigrationsError;
use diesel::prelude::*;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type Database = Arc<Mutex<SqliteConnection>>;

embed_migrations!("migrations/");

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

    static ref DB_PATH: PathBuf = {
        HAMMOND_XDG.place_data_file("hammond.db").unwrap()
        };

    pub static ref DL_DIR: PathBuf = {
        HAMMOND_XDG.create_data_directory("Downloads").unwrap()
    };
}

pub fn init() -> Result<(), RunMigrationsError> {
    let conn = establish_connection();
    run_migration_on(&conn)
}

pub fn run_migration_on(connection: &SqliteConnection) -> Result<(), RunMigrationsError> {
    info!("Running DB Migrations...");
    embedded_migrations::run(connection)
    // embedded_migrations::run_with_output(connection, &mut std::io::stdout())
}

pub fn establish_connection() -> SqliteConnection {
    let database_url = DB_PATH.to_str().unwrap();
    SqliteConnection::establish(database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
