use diesel;
use diesel::r2d2;
use diesel_migrations::RunMigrationsError;
use hyper;
use native_tls;
use reqwest;
// use rss;
use url;

use std::io;

#[allow(dead_code)]
#[derive(Fail, Debug)]
#[fail(display = "IO Error: {}", _0)]
struct IOError(io::Error);

// fadsadfs NOT SYNC
// #[derive(Fail, Debug)]
// #[fail(display = "RSS Error: {}", _0)]
// struct RSSError(rss::Error);

#[derive(Fail, Debug)]
pub enum DatabaseError {
    #[fail(display = "SQL Query failed: {}", _0)] DieselResultError(diesel::result::Error),
    #[fail(display = "Database Migration error: {}", _0)] DieselMigrationError(RunMigrationsError),
    #[fail(display = "R2D2 error: {}", _0)] R2D2Error(r2d2::Error),
    #[fail(display = "R2D2 Pool error: {}", _0)] R2D2PoolError(r2d2::PoolError),
}

#[derive(Fail, Debug)]
pub enum HttpError {
    #[fail(display = "Reqwest Error: {}", _0)] ReqError(reqwest::Error),
    #[fail(display = "Hyper Error: {}", _0)] HyperError(hyper::Error),
    #[fail(display = "Url Error: {}", _0)] UrlError(url::ParseError),
    #[fail(display = "TLS Error: {}", _0)] TLSError(native_tls::Error),
}
