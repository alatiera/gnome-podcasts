use diesel;
use diesel::r2d2;
use diesel_migrations::RunMigrationsError;
use hyper;
use native_tls;
use reqwest;
use rss;
use url;

use std::io;

error_chain! {
    foreign_links {
        DieselResultError(diesel::result::Error);
        DieselMigrationError(RunMigrationsError);
        R2D2Error(r2d2::Error);
        R2D2PoolError(r2d2::PoolError);
        RSSError(rss::Error);
        ReqError(reqwest::Error);
        HyperError(hyper::Error);
        UrlError(url::ParseError);
        TLSError(native_tls::Error);
        IoError(io::Error);
    }
}

#[derive(Fail, Debug)]
pub enum DatabaseError {
    #[fail(display = "SQL Query failed: {}", _0)] DieselResultError(diesel::result::Error),
    #[fail(display = "Database Migration error: {}", _0)] DieselMigrationError(RunMigrationsError),
    #[fail(display = "R2D2 error: {}", _0)] R2D2Error(r2d2::Error),
    #[fail(display = "R2D2 Pool error: {}", _0)] R2D2PoolError(r2d2::PoolError),
}
