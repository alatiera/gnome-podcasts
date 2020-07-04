// errors.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use diesel;
use diesel::r2d2;
use diesel_migrations::RunMigrationsError;
use http;
use hyper;
use native_tls;
use reqwest;
use rss;
use url;
use xml;

use std::io;

use crate::models::Source;

#[fail(
    display = "Request to {} returned {}. Context: {}",
    url, status_code, context
)]
#[derive(Fail, Debug)]
pub struct HttpStatusError {
    url: String,
    status_code: hyper::StatusCode,
    context: String,
}

impl HttpStatusError {
    pub fn new(url: String, code: hyper::StatusCode, context: String) -> Self {
        HttpStatusError {
            url,
            status_code: code,
            context,
        }
    }
}

#[derive(Fail, Debug)]
pub enum DataError {
    #[fail(display = "SQL Query failed: {}", _0)]
    DieselResultError(#[cause] diesel::result::Error),
    #[fail(display = "Database Migration error: {}", _0)]
    DieselMigrationError(#[cause] RunMigrationsError),
    #[fail(display = "R2D2 error: {}", _0)]
    R2D2Error(#[cause] r2d2::Error),
    #[fail(display = "R2D2 Pool error: {}", _0)]
    R2D2PoolError(#[cause] r2d2::PoolError),
    #[fail(display = "Hyper Error: {}", _0)]
    HyperError(#[cause] hyper::Error),
    #[fail(display = "ToStr Error: {}", _0)]
    HttpToStr(#[cause] http::header::ToStrError),
    #[fail(display = "Failed to parse a url: {}", _0)]
    UrlError(#[cause] url::ParseError),
    #[fail(display = "TLS Error: {}", _0)]
    TLSError(#[cause] native_tls::Error),
    #[fail(display = "IO Error: {}", _0)]
    IOError(#[cause] io::Error),
    #[fail(display = "RSS Error: {}", _0)]
    RssError(#[cause] rss::Error),
    #[fail(display = "XML Reader Error: {}", _0)]
    XmlReaderError(#[cause] xml::reader::Error),
    #[fail(display = "Error: {}", _0)]
    Bail(String),
    #[fail(display = "{}", _0)]
    HttpStatusGeneral(HttpStatusError),
    #[fail(display = "Source redirects to a new url")]
    FeedRedirect(Source),
    #[fail(display = "Feed is up to date")]
    FeedNotModified(Source),
    #[fail(
        display = "Error occurred while Parsing an Episode. Reason: {}",
        reason
    )]
    ParseEpisodeError { reason: String, parent_id: i32 },
    #[fail(display = "Episode was not changed and thus skipped.")]
    EpisodeNotChanged,
    #[fail(display = "Invalid Uri Error: {}", _0)]
    InvalidUri(#[cause] http::uri::InvalidUri),
}

// Maps a type to a variant of the DataError enum
macro_rules! easy_from_impl {
    ($outer_type:ty, $from:ty => $to:expr) => (
        impl From<$from> for $outer_type {
            fn from(err: $from) -> Self {
                $to(err)
            }
        }
    );
    ($outer_type:ty, $from:ty => $to:expr, $($f:ty => $t:expr),+) => (
        easy_from_impl!($outer_type, $from => $to);
        easy_from_impl!($outer_type, $($f => $t),+);
    );
}

easy_from_impl!(
    DataError,
    RunMigrationsError       => DataError::DieselMigrationError,
    diesel::result::Error    => DataError::DieselResultError,
    r2d2::Error              => DataError::R2D2Error,
    r2d2::PoolError          => DataError::R2D2PoolError,
    hyper::Error             => DataError::HyperError,
    http::header::ToStrError => DataError::HttpToStr,
    url::ParseError          => DataError::UrlError,
    native_tls::Error        => DataError::TLSError,
    io::Error                => DataError::IOError,
    rss::Error               => DataError::RssError,
    xml::reader::Error       => DataError::XmlReaderError,
    http::uri::InvalidUri    => DataError::InvalidUri,
    String                   => DataError::Bail
);

#[derive(Fail, Debug)]
pub enum DownloadError {
    #[fail(display = "Reqwest error: {}", _0)]
    RequestError(#[cause] reqwest::Error),
    #[fail(display = "Data error: {}", _0)]
    DataError(#[cause] DataError),
    #[fail(display = "Io error: {}", _0)]
    IoError(#[cause] io::Error),
    #[fail(display = "Unexpected server response: {}", _0)]
    UnexpectedResponse(reqwest::StatusCode),
    #[fail(display = "The Download was cancelled.")]
    DownloadCancelled,
    #[fail(display = "Remote Image location not found.")]
    NoImageLocation,
    #[fail(display = "Failed to parse CacheLocation.")]
    InvalidCacheLocation,
    #[fail(display = "Failed to parse Cached Image Location.")]
    InvalidCachedImageLocation,
}

easy_from_impl!(
    DownloadError,
    reqwest::Error => DownloadError::RequestError,
    io::Error => DownloadError::IoError,
    DataError => DownloadError::DataError
);
