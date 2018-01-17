use diesel;
use diesel_migrations::RunMigrationsError;
use hyper;
use native_tls;
use r2d2;
use reqwest;
use rss;

use std::io;

error_chain! {
    foreign_links {
        R2D2Error(r2d2::Error);
        DieselResultError(diesel::result::Error);
        DieselMigrationError(RunMigrationsError);
        RSSError(rss::Error);
        ReqError(reqwest::Error);
        HyperError(hyper::Error);
        TLSError(native_tls::Error);
        IoError(io::Error);
    }
}
