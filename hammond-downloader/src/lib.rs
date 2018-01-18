#![recursion_limit = "1024"]

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

#[cfg(test)]
extern crate hyper_tls;
#[cfg(test)]
extern crate tokio_core;
