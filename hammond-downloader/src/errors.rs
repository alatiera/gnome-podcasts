use hammond_data;
use reqwest;
use std::io;

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(io::Error);
        DataError(hammond_data::errors::Error);
    }
}
