// lib.rs
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

#![recursion_limit = "1024"]

#[cfg(test)]
#[macro_use]
extern crate maplit;

#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;

pub mod database;
#[allow(missing_docs)]
pub mod dbqueries;
pub mod discovery;
#[allow(missing_docs)]
pub mod downloader;
#[allow(missing_docs)]
pub mod errors;
mod feed;
pub mod feed_manager;
pub(crate) mod models;
/// Login and `sync` functions for nextcloud sychronization via the [GPodder sync addon API](https://github.com/thrillfall/nextcloud-gpodder)
pub mod nextcloud_sync;
pub mod opml;
mod parser;
pub mod pipeline;
mod schema;
#[cfg(test)]
pub mod test_feeds;
pub mod utils;

pub use crate::feed::{Feed, FeedBuilder};
pub use crate::feed_manager::*;
pub use crate::models::Save;
/// Sync datatypes to store updates that still have to be sent out.
/// This is mostly glue code for the DB, use store(), fetch(), delete() methods to interact.
pub use crate::models::sync;
pub use crate::models::{
    Episode, EpisodeCleanerModel, EpisodeId, EpisodeModel, EpisodeWidgetModel, Show,
    ShowCoverModel, ShowId, Source, SourceId,
};

// Set the user agent, See #53 for more
// Keep this in sync with Tor-browser releases
/// The user-agent to be used for all the requests.
/// It originates from the Tor-browser UA.
pub const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";
/// Used by nextcloud to display the Client name in the password list.
/// A proper name helps users to not revoke the wrong entry, when cleaning up passwords.
pub const USER_AGENT_NEXTCLOUD: &str = "Gnome Podcasts - Nextcloud Sync";

/// [XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) Paths.
pub mod xdg_dirs {
    use std::path::PathBuf;
    use std::sync::LazyLock;

    pub(crate) static PODCASTS_XDG: LazyLock<xdg::BaseDirectories> =
        LazyLock::new(|| xdg::BaseDirectories::with_prefix("gnome-podcasts"));

    /// XDG_DATA Directory `Pathbuf`.
    pub static PODCASTS_DATA: LazyLock<PathBuf> = LazyLock::new(|| {
        PODCASTS_XDG
            .create_data_directory(PODCASTS_XDG.get_data_home().unwrap())
            .unwrap()
    });

    /// XDG_CONFIG Directory `Pathbuf`.
    pub static PODCASTS_CONFIG: LazyLock<PathBuf> = LazyLock::new(|| {
        PODCASTS_XDG
            .create_config_directory(PODCASTS_XDG.get_config_home().unwrap())
            .unwrap()
    });

    /// XDG_CACHE Directory `Pathbuf`.
    pub static PODCASTS_CACHE: LazyLock<PathBuf> = LazyLock::new(|| {
        PODCASTS_XDG
            .create_cache_directory(PODCASTS_XDG.get_cache_home().unwrap())
            .unwrap()
    });

    /// GNOME Podcasts Download Directory `PathBuf`.
    pub static DL_DIR: LazyLock<PathBuf> =
        LazyLock::new(|| PODCASTS_XDG.create_data_directory("Downloads").unwrap());

    /// GNOME Podcasts Tmp Directory `PathBuf`.
    pub static TMP_DIR: LazyLock<PathBuf> =
        LazyLock::new(|| PODCASTS_XDG.create_data_directory("tmp").unwrap());
}
