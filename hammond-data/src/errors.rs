use diesel::migrations::RunMigrationsError;
use diesel::result;
use rss;
use hyper;
use reqwest;

use std::io;

error_chain! {
    foreign_links {
        MigrationError(RunMigrationsError);
        DieselResultError(result::Error);
        RSSError(rss::Error);
        HyperError(hyper::error::Error);
        ReqError(reqwest::Error);
        IoError(io::Error);
    }
}
