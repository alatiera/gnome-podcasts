use diesel;
use diesel::r2d2;
use diesel_migrations::RunMigrationsError;
use hyper;
use native_tls;
// use rss;
use url;

use std::io;
// use std::fmt;

// fadsadfs NOT SYNC
// #[derive(Fail, Debug)]
// #[fail(display = "RSS Error: {}", _0)]
// struct RSSError(rss::Error);

#[derive(Fail, Debug)]
pub enum DataError {
    #[fail(display = "SQL Query failed: {}", _0)]
    DieselResultError(#[cause] diesel::result::Error),
    #[fail(display = "Database Migration error: {}", _0)]
    DieselMigrationError(#[cause] RunMigrationsError),
    #[fail(display = "R2D2 error: {}", _0)]
    R2D2Error(#[cause] r2d2::Error),
    #[fail(display = "R2D2 Pool error: {}", _0)]
    R2D2PoolError(#[cause] r2d2::PoolError),
    #[fail(display = "Hyper Error: {}", _0)]
    HyperError(#[cause] hyper::Error),
    #[fail(display = "Failed to parse a url: {}", _0)]
    // TODO: print the url too
    UrlError(#[cause] url::ParseError),
    #[fail(display = "TLS Error: {}", _0)]
    TLSError(#[cause] native_tls::Error),
    #[fail(display = "IO Error: {}", _0)]
    IOError(#[cause] io::Error),
    #[fail(display = "RSS Error: {}", _0)]
    // Rss::Error is not yet Sync
    RssCrateError(String),
    #[fail(display = "Error: {}", _0)]
    Bail(String),
    #[fail(display = "Request to {} returned {}. Context: {}", url, status_code, context)]
    HttpStatusError {
        url: String,
        status_code: hyper::StatusCode,
        context: String,
    },
    #[fail(display = "Error occured while Parsing an Episode. Reason: {}", reason)]
    ParseEpisodeError { reason: String, parent_id: i32 },
    #[fail(display = "No Futures where produced to be run.")]
    EmptyFuturesList,
    #[fail(display = "Episode was not changed and thus skipped.")]
    EpisodeNotChanged,
}

impl From<RunMigrationsError> for DataError {
    fn from(err: RunMigrationsError) -> Self {
        DataError::DieselMigrationError(err)
    }
}

impl From<diesel::result::Error> for DataError {
    fn from(err: diesel::result::Error) -> Self {
        DataError::DieselResultError(err)
    }
}

impl From<r2d2::Error> for DataError {
    fn from(err: r2d2::Error) -> Self {
        DataError::R2D2Error(err)
    }
}

impl From<r2d2::PoolError> for DataError {
    fn from(err: r2d2::PoolError) -> Self {
        DataError::R2D2PoolError(err)
    }
}

impl From<hyper::Error> for DataError {
    fn from(err: hyper::Error) -> Self {
        DataError::HyperError(err)
    }
}

impl From<url::ParseError> for DataError {
    fn from(err: url::ParseError) -> Self {
        DataError::UrlError(err)
    }
}

impl From<native_tls::Error> for DataError {
    fn from(err: native_tls::Error) -> Self {
        DataError::TLSError(err)
    }
}

impl From<io::Error> for DataError {
    fn from(err: io::Error) -> Self {
        DataError::IOError(err)
    }
}

impl From<String> for DataError {
    fn from(err: String) -> Self {
        DataError::Bail(err)
    }
}
