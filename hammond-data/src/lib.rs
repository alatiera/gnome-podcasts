#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;
extern crate loggerv;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

extern crate chrono;
extern crate hyper;
extern crate rayon;
extern crate regex;
extern crate reqwest;
extern crate rfc822_sanitizer;
extern crate rss;
extern crate time;
extern crate xdg;

pub mod dbqueries;
pub mod models;
pub mod schema;

pub mod errors {

    use reqwest;
    use rss;
    use chrono;
    use hyper;
    use time;
    use diesel::migrations::RunMigrationsError;
    use diesel::result;
    use regex;

    use std::io;
    // use std::option;
    // use std::sync;

    error_chain! {
        foreign_links {
            ReqError(reqwest::Error);
            IoError(io::Error);
            Log(::log::SetLoggerError);
            MigrationError(RunMigrationsError);
            RSSError(rss::Error);
            DieselResultError(result::Error);
            ChronoError(chrono::ParseError);
            DurationError(time::OutOfRangeError);
            HyperError(hyper::error::Error);
            RegexError(regex::Error);
            // NoneError(option::NoneError);
            // MutexPoison(sync::PoisonError);
        }
    }
}

use errors::*;
use diesel::prelude::*;
use std::path::PathBuf;

embed_migrations!("migrations/");

lazy_static!{

    static ref HAMMOND_XDG: xdg::BaseDirectories = {
        xdg::BaseDirectories::with_prefix("Hammond").unwrap()
    };

    static ref HAMMOND_DATA: PathBuf = {
        HAMMOND_XDG.create_data_directory(HAMMOND_XDG.get_data_home()).unwrap()
    };

    static ref _HAMMOND_CONFIG: PathBuf = {
        HAMMOND_XDG.create_config_directory(HAMMOND_XDG.get_config_home()).unwrap()
    };

    static ref _HAMMOND_CACHE: PathBuf = {
        HAMMOND_XDG.create_cache_directory(HAMMOND_XDG.get_cache_home()).unwrap()
    };

    static ref DB_PATH: PathBuf = {
        // Ensure that xdg_data is created.
        &HAMMOND_DATA;

        HAMMOND_XDG.place_data_file("hammond.db").unwrap()
        };

    pub static ref DL_DIR: PathBuf = {
        &HAMMOND_DATA;
        HAMMOND_XDG.create_data_directory("Downloads").unwrap()
    };
}

// TODO: REFACTOR
pub fn init() -> Result<()> {
    let conn = establish_connection();
    // embedded_migrations::run(&conn)?;
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout())?;

    Ok(())
}

pub fn run_migration_on(connection: &SqliteConnection) -> Result<()> {
    embedded_migrations::run_with_output(connection, &mut std::io::stdout())?;
    Ok(())
}

pub fn establish_connection() -> SqliteConnection {
    let database_url = DB_PATH.to_str().unwrap();
    // let database_url = &String::from(".random/foo.db");
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
