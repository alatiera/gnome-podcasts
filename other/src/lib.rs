#![feature(use_extern_macros)]

extern crate diesel;
extern crate hammond_data;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate rayon;
extern crate reqwest;
extern crate rfc822_sanitizer;
extern crate rss;

pub mod feedparser;
pub mod downloader;
pub mod index_feed;
