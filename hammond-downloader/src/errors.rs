use diesel::result;
use reqwest;
use std::io;

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(io::Error);
        DieselResultError(result::Error);
    }
}
