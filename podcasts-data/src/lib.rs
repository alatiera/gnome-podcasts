#![recursion_limit = "1024"]
#![allow(unknown_lints)]
#![cfg_attr(
    all(test, feature = "clippy"),
    allow(option_unwrap_used, result_unwrap_used)
)]
#![cfg_attr(feature = "cargo-clippy", allow(option_map_unit_fn))]
#![cfg_attr(
    feature = "clippy",
    warn(
        option_unwrap_used,
        result_unwrap_used,
        print_stdout,
        wrong_pub_self_convention,
        mut_mut,
        non_ascii_literal,
        similar_names,
        unicode_not_nfc,
        enum_glob_use,
        if_not_else,
        items_after_statements,
        used_underscore_binding
    )
)]
// Enable lint group collections
#![warn(nonstandard_style, bad_style, unused)]
#![allow(edition_2018, rust_2018_idioms)]
// standalone lints
#![warn(
    const_err,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    plugin_as_library,
    private_no_mangle_fns,
    private_no_mangle_statics,
    unconditional_recursion,
    unions_with_drop_fields,
    while_true,
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    elided_lifetime_in_paths,
    missing_copy_implementations
)]
#![allow(proc_macro_derive_resolution_fallback)]
#![deny(warnings)]

//! FIXME: Docs

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[cfg(test)]
#[macro_use]
extern crate maplit;

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
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate num_cpus;
extern crate rayon;
extern crate rfc822_sanitizer;
extern crate rss;
extern crate tokio_core;
extern crate tokio_executor;
extern crate tokio_threadpool;
extern crate url;
extern crate xdg;
extern crate xml;

pub mod database;
#[allow(missing_docs)]
pub mod dbqueries;
#[allow(missing_docs)]
pub mod errors;
mod feed;
pub(crate) mod models;
pub mod opml;
mod parser;
pub mod pipeline;
mod schema;
pub mod utils;

pub use feed::{Feed, FeedBuilder};
pub use models::Save;
pub use models::{Episode, EpisodeWidgetModel, Show, ShowCoverModel, Source};

// Set the user agent, See #53 for more
// Keep this in sync with Tor-browser releases
/// The user-agent to be used for all the requests.
/// It originates from the Tor-browser UA.
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 6.1; rv:52.0) Gecko/20100101 Firefox/52.0";

/// [XDG Base Direcotory](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) Paths.
#[allow(missing_debug_implementations)]
pub mod xdg_dirs {
    use std::path::PathBuf;
    use xdg;

    lazy_static!{
        pub(crate) static ref PODCASTS_XDG: xdg::BaseDirectories = {
            xdg::BaseDirectories::with_prefix("gnome-podcasts").unwrap()
        };

        /// XDG_DATA Directory `Pathbuf`.
        pub static ref PODCASTS_DATA: PathBuf = {
            PODCASTS_XDG.create_data_directory(PODCASTS_XDG.get_data_home()).unwrap()
        };

        /// XDG_CONFIG Directory `Pathbuf`.
        pub static ref PODCASTS_CONFIG: PathBuf = {
            PODCASTS_XDG.create_config_directory(PODCASTS_XDG.get_config_home()).unwrap()
        };

        /// XDG_CACHE Directory `Pathbuf`.
        pub static ref PODCASTS_CACHE: PathBuf = {
            PODCASTS_XDG.create_cache_directory(PODCASTS_XDG.get_cache_home()).unwrap()
        };

        /// GNOME Podcasts Download Direcotry `PathBuf`.
        pub static ref DL_DIR: PathBuf = {
            PODCASTS_XDG.create_data_directory("Downloads").unwrap()
        };
    }
}
