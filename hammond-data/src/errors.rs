use diesel::result;
use diesel::migrations::RunMigrationsError;
use rss;
use reqwest;
use r2d2;

use std::io;

error_chain! {
    foreign_links {
        R2D2TimeoutError(r2d2::GetTimeout);
        DieselResultError(result::Error);
        DieselMigrationError(RunMigrationsError);
        RSSError(rss::Error);
        ReqError(reqwest::Error);
        IoError(io::Error);
    }
}
