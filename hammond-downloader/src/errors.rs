use reqwest;
use hammond_data;
use std::io;

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(io::Error);
        DataError(hammond_data::errors::Error);
    }
}
