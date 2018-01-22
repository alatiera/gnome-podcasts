use diesel;
use diesel::r2d2;
use diesel_migrations::RunMigrationsError;
use hyper;
use native_tls;
use reqwest;
use rss;

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
        TLSError(native_tls::Error);
        IoError(io::Error);
    }
}
