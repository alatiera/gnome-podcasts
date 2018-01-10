#![recursion_limit = "1024"]
#![cfg_attr(feature = "cargo-clippy", allow(blacklisted_name))]
#![cfg_attr(feature = "clippy",
            warn(option_unwrap_used, result_unwrap_used, print_stdout,
                 wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names,
                 unicode_not_nfc, enum_glob_use, if_not_else, items_after_statements,
                 used_underscore_binding))]
#![cfg_attr(all(test, feature = "clippy"), allow(option_unwrap_used, result_unwrap_used))]

//! A libraty for parsing, indexing and retrieving podcast Feeds,
//! into and from a Database.

#![allow(unknown_lints)]
#![deny(bad_style, const_err, dead_code, improper_ctypes, legacy_directory_ownership,
        non_shorthand_field_patterns, no_mangle_generic_items, overflowing_literals,
        path_statements, patterns_in_fns_without_body, plugin_as_library, private_in_public,
        private_no_mangle_fns, private_no_mangle_statics, safe_extern_statics,
        unconditional_recursion, unions_with_drop_fields, unused, unused_allocation,
        unused_comparisons, unused_parens, while_true)]
#![deny(missing_debug_implementations, missing_docs, trivial_casts, trivial_numeric_casts,
        unused_extern_crates)]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate derive_builder;

extern crate ammonia;
extern crate chrono;
extern crate itertools;
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
pub mod database;
pub(crate) mod models;
mod parser;
mod schema;

pub use models::queryables::{Episode, EpisodeWidgetQuery, Podcast, PodcastCoverQuery, Source};

/// [XDG Base Direcotory](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) Paths.
#[allow(missing_debug_implementations)]
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
