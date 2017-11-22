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
pub mod feed;
pub mod errors;
pub mod database;
mod models;
mod parser;
mod schema;

pub use models::{Episode, Podcast, Source};

pub mod xdg_ {
    use std::path::PathBuf;
    use xdg;

    lazy_static!{
        pub static ref HAMMOND_XDG: xdg::BaseDirectories = {
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
    }
}
