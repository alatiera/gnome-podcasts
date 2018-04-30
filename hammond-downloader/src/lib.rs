#![recursion_limit = "1024"]
#![warn(unused_extern_crates, unused)]
#![allow(unknown_lints)]
#![cfg_attr(feature = "cargo-clippy", allow(blacklisted_name, option_map_unit_fn))]
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
extern crate hammond_data;
extern crate hyper;
extern crate mime_guess;
extern crate reqwest;
extern crate tempdir;

pub mod downloader;
pub mod errors;
