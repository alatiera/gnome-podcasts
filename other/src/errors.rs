use reqwest;
use rss;
use hyper;
use diesel::result;
use hammond_data;

use std::io;

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(io::Error);
        Log(::log::SetLoggerError);
        RSSError(rss::Error);
        DieselResultError(result::Error);
        HyperError(hyper::error::Error);
        HamDBError(hammond_data::errors::Error);
    }
}
