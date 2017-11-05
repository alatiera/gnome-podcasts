use diesel::result;
use rss;
use reqwest;

use std::io;

error_chain! {
    foreign_links {
        DieselResultError(result::Error);
        RSSError(rss::Error);
        ReqError(reqwest::Error);
        IoError(io::Error);
    }
}
