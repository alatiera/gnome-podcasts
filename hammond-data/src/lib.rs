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
extern crate xdg;

pub mod dbqueries;
pub mod utils;
pub mod models;
pub mod feed;
pub mod errors;
mod parser;
mod schema;

use std::path::PathBuf;

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

    pub static ref DB_PATH: PathBuf = HAMMOND_XDG.place_data_file("hammond.db").unwrap();
}

#[cfg(not(test))]
lazy_static! {
    pub static ref POOL: utils::Pool = utils::init_pool(DB_PATH.to_str().unwrap());
}

#[cfg(test)]
lazy_static! {
    static ref TEMPDB: TempDB =  get_temp_db();

    pub static ref POOL: &'static utils::Pool = &TEMPDB.2;
}

#[cfg(test)]
struct TempDB(tempdir::TempDir, PathBuf, utils::Pool);

#[cfg(test)]
extern crate rand;
#[cfg(test)]
extern crate tempdir;
#[cfg(test)]
use rand::Rng;

#[cfg(test)]
/// Create and return a Temporary DB.
/// Will be destroed once the returned variable(s) is dropped.
fn get_temp_db() -> TempDB {
    let mut rng = rand::thread_rng();

    let tmp_dir = tempdir::TempDir::new("hammond_unit_test").unwrap();
    let db_path = tmp_dir
        .path()
        .join("test.db");

    let pool = utils::init_pool(db_path.to_str().unwrap());
    let db = pool.clone().get().unwrap();
    utils::run_migration_on(&*db).unwrap();

    TempDB(tmp_dir, db_path, pool)
}
