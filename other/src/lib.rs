#![recursion_limit = "1024"]

extern crate diesel;
extern crate hammond_data;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate rayon;
extern crate reqwest;
extern crate rfc822_sanitizer;
extern crate rss;
#[macro_use]
extern crate error_chain;

pub mod feedparser;
pub mod downloader;
pub mod index_feed;
pub mod errors;