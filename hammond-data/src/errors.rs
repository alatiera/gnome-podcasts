use diesel::result;
use diesel_migrations::RunMigrationsError;
use rss;
use reqwest;
use r2d2;

use std::io;

error_chain! {
    foreign_links {
        R2D2Error(r2d2::Error);
        DieselResultError(result::Error);
        DieselMigrationError(RunMigrationsError);
        RSSError(rss::Error);
        ReqError(reqwest::Error);
        IoError(io::Error);
    }
}
