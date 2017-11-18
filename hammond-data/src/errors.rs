use diesel::result;
use diesel::migrations::RunMigrationsError;
use rss;
use reqwest;

use std::io;

error_chain! {
    foreign_links {
        DieselResultError(result::Error);
        DieselMigrationError(RunMigrationsError);
        RSSError(rss::Error);
        ReqError(reqwest::Error);
        IoError(io::Error);
    }
}
