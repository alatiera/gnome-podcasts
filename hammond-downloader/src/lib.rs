#![recursion_limit = "1024"]
#![deny(unused_extern_crates, unused)]
#![allow(unknown_lints)]
#![cfg_attr(feature = "cargo-clippy", allow(blacklisted_name))]

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
