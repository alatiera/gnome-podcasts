use hammond_data;
use reqwest;
use std::io;

#[derive(Fail, Debug)]
pub enum DownloaderError {
    #[fail(display = "Reqwest error: {}", _0)]
    RequestError(reqwest::Error),
    #[fail(display = "Data error: {}", _0)]
    DataError(hammond_data::errors::DataError),
    #[fail(display = "Io error: {}", _0)]
    IoError(io::Error),
}
