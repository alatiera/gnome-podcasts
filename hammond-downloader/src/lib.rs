#![recursion_limit = "1024"]

extern crate diesel;
#[macro_use]
extern crate error_chain;
extern crate hammond_data;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate mime;
extern crate rand;
extern crate reqwest;
// extern crate rss;
extern crate tempdir;

pub mod downloader;
pub mod errors;
