#![recursion_limit = "1024"]
#![warn(missing_docs)]
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

#[allow(missing_docs)]
pub mod dbqueries;
pub mod utils;
pub mod feed;
#[allow(missing_docs)]
pub mod errors;
pub(crate) mod database;
pub(crate) mod models;
mod parser;
mod schema;

pub use models::queryables::{Episode, Podcast, Source};

/// [XDG Base Direcotory](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) Paths.
pub mod xdg_dirs {
    use std::path::PathBuf;
    use xdg;

    lazy_static!{
        pub(crate) static ref HAMMOND_XDG: xdg::BaseDirectories = {
            xdg::BaseDirectories::with_prefix("hammond").unwrap()
        };

        /// XDG_DATA Directory `Pathbuf`.
        pub static ref HAMMOND_DATA: PathBuf = {
            HAMMOND_XDG.create_data_directory(HAMMOND_XDG.get_data_home()).unwrap()
        };

        /// XDG_CONFIG Directory `Pathbuf`.
        pub static ref HAMMOND_CONFIG: PathBuf = {
            HAMMOND_XDG.create_config_directory(HAMMOND_XDG.get_config_home()).unwrap()
        };

        /// XDG_CACHE Directory `Pathbuf`.
        pub static ref HAMMOND_CACHE: PathBuf = {
            HAMMOND_XDG.create_cache_directory(HAMMOND_XDG.get_cache_home()).unwrap()
        };

        /// Hammond Download Direcotry `PathBuf`.
        pub static ref DL_DIR: PathBuf = {
            HAMMOND_XDG.create_data_directory("Downloads").unwrap()
        };
    }
}
