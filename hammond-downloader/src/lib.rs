#![recursion_limit = "1024"]

extern crate diesel;
#[macro_use]
extern crate error_chain;
extern crate glob;
extern crate hammond_data;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate mime_guess;
extern crate reqwest;
extern crate tempdir;

pub mod downloader;
pub mod errors;
