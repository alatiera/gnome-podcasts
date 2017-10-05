use diesel::migrations::RunMigrationsError;
use diesel::result;
use rss;
use hyper;
use reqwest;
use log;

use std::io;

error_chain! {
    foreign_links {
        LogError(log::SetLoggerError);
        MigrationError(RunMigrationsError);
        DieselResultError(result::Error);
        RSSError(rss::Error);
        HyperError(hyper::error::Error);
        ReqError(reqwest::Error);
        IoError(io::Error);
    }
}
