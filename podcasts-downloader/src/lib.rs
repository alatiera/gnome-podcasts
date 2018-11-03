#![recursion_limit = "1024"]
#![allow(unknown_lints)]
#![cfg_attr(feature = "cargo-clippy", allow(blacklisted_name, option_map_unit_fn))]
// Enable lint group collections
#![warn(nonstandard_style, edition_2018, rust_2018_idioms, bad_style, unused)]
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
    trivial_casts,
    trivial_numeric_casts,
    elided_lifetime_in_paths,
    missing_copy_implementations
)]
#![deny(warnings)]

extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

extern crate glob;
extern crate mime_guess;
#[macro_use]
extern crate podcasts_data;
extern crate reqwest;
extern crate tempdir;

pub mod downloader;
pub mod errors;
