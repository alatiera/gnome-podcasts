#![recursion_limit = "1024"]
#![cfg_attr(all(test, feature = "clippy"), allow(option_unwrap_used, result_unwrap_used))]
#![cfg_attr(feature = "cargo-clippy", allow(blacklisted_name))]
#![cfg_attr(feature = "clippy",
            warn(option_unwrap_used, result_unwrap_used, print_stdout,
                 wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names,
                 unicode_not_nfc, enum_glob_use, if_not_else, items_after_statements,
                 used_underscore_binding))]
#![allow(unknown_lints)]
#![deny(bad_style, const_err, dead_code, improper_ctypes, legacy_directory_ownership,
        non_shorthand_field_patterns, no_mangle_generic_items, overflowing_literals,
        path_statements, patterns_in_fns_without_body, plugin_as_library, private_in_public,
        private_no_mangle_fns, private_no_mangle_statics, safe_extern_statics,
        unconditional_recursion, unions_with_drop_fields, unused_allocation, unused_comparisons,
        unused_parens, while_true)]
#![deny(missing_debug_implementations, missing_docs, trivial_casts, trivial_numeric_casts)]
#![deny(unused_extern_crates, unused)]

// #![feature(conservative_impl_trait)]

//! FIXME: Docs

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
// #[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

extern crate ammonia;
extern crate chrono;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate hyper_tls;
extern crate itertools;
extern crate native_tls;
extern crate num_cpus;
extern crate rayon;
extern crate rfc822_sanitizer;
extern crate rss;
extern crate tokio_core;
extern crate url;
extern crate xdg;

#[allow(missing_docs)]
pub mod dbqueries;
#[allow(missing_docs)]
pub mod errors;
pub mod utils;
pub mod database;
pub mod pipeline;
pub(crate) mod models;
mod feed;
mod parser;
mod schema;

pub use feed::{Feed, FeedBuilder};
pub use models::{Episode, EpisodeWidgetQuery, Podcast, PodcastCoverQuery, Source};
pub use models::Save;

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
