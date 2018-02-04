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

#[derive(Fail, Debug)]
enum DownloaderError {
    #[fail(display = "Reqwest error: {}", _0)] RequestError(reqwest::Error),
    // NOT SYNC.
    // #[fail(display = "Data error: {}", _0)]
    // DataError(hammond_data::errors::Error),
    #[fail(display = "Io error: {}", _0)] IoError(io::Error),
}
