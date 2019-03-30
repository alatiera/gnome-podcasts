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


#![allow(bare_trait_objects)]

use podcasts_data::errors::DataError;
use reqwest;
use std::io;

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
    DownloadError,
    reqwest::Error => DownloadError::RequestError,
    io::Error => DownloadError::IoError,
    DataError => DownloadError::DataError
);
