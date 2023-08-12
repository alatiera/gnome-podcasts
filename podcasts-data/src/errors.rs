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

use diesel::r2d2;

use std::io;

use crate::models::{ShowId, Source};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataError {
    #[error("SQL Query failed: {0}")]
    DieselResultError(#[from] diesel::result::Error),
    #[error("Database Migration error")]
    DieselMigrationError,
    #[error("R2D2 error: {0}")]
    R2D2Error(#[from] r2d2::Error),
    #[error("R2D2 Pool error: {0}")]
    R2D2PoolError(#[from] r2d2::PoolError),
    #[error("ToStr Error: {0}")]
    HttpToStr(#[from] http::header::ToStrError),
    #[error("Failed to parse a url: {0}")]
    UrlError(#[from] url::ParseError),
    #[error("TLS Error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("IO Error: {0}")]
    IOError(#[from] io::Error),
    #[error("RSS Error: {0}")]
    RssError(#[from] rss::Error),
    #[error("XML Reader Error: {0}")]
    XmlReaderError(#[from] xml::reader::Error),
    #[error("Error: {0}")]
    Bail(String),
    #[error("Request to {url} returned {status_code}. Context: {context}")]
    HttpStatusGeneral {
        url: String,
        status_code: reqwest::StatusCode,
        context: String,
    },
    #[error("Source redirects to a new url")]
    FeedRedirect(Source),
    #[error("Feed is up to date")]
    FeedNotModified(Source),
    #[error("Error occurred while Parsing an Episode. Reason: {}", reason)]
    ParseEpisodeError { reason: String, parent_id: ShowId },
    #[error("Episode was not changed and thus skipped.")]
    EpisodeNotChanged,
    #[error("Invalid Uri Error: {0}")]
    InvalidUri(#[from] http::uri::InvalidUri),
    #[error("Builder error: {0}")]
    BuilderError(String),
    #[error("keyring error: {0}")]
    KeyringError(#[from] oo7::Error),
    #[error("from_utf8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Data error: {0}")]
    DataError(#[from] DataError),
    #[error("Io error: {0}")]
    IoError(#[from] io::Error),
    #[error("Unexpected server response: {0}")]
    UnexpectedResponse(reqwest::StatusCode),
    #[error("The Download was cancelled.")]
    DownloadCancelled,
    #[error("Remote Image location not found.")]
    NoImageLocation,
    #[error("Failed to parse CacheLocation.")]
    InvalidCacheLocation,
    #[error("Failed to parse Cached Image Location.")]
    InvalidCachedImageLocation,
    #[error("Download no longer needed.")]
    NoLongerNeeded,
}
