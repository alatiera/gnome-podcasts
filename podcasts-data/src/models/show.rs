// show.rs
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

use crate::errors::DataError;
use crate::models::Source;
use crate::schema::shows;

use crate::database::connection;
use crate::utils::calculate_hash;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::query_dsl::filter_dsl::FilterDsl;
use diesel::{ExpressionMethods, RunQueryDsl};

#[derive(Queryable, Identifiable, AsChangeset, Associations, PartialEq)]
#[belongs_to(Source, foreign_key = "source_id")]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "shows"]
#[derive(Debug, Clone)]
/// Diesel Model of the shows table.
pub struct Show {
    id: i32,
    title: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    image_uri_hash: Option<i64>,
    image_cached: NaiveDateTime,
    source_id: i32,
}

impl Show {
    /// Get the Feed `id`.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the Feed `title`.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the Feed `link`.
    ///
    /// Usually the website/homepage of the content creator.
    pub fn link(&self) -> &str {
        &self.link
    }

    /// Get the `description`.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the `image_uri`.
    ///
    /// Represents the uri(url usually) that the Feed cover image is located at.
    pub fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_ref().map(|s| s.as_str())
    }

    /// Get the `image_uri_hash`.
    pub fn image_uri_hash(&self) -> Option<i64> {
        self.image_uri_hash
    }

    /// Get the `image_cached`.
    pub fn image_cached(&self) -> &NaiveDateTime {
        &self.image_cached
    }

    /// `Source` table foreign key.
    pub fn source_id(&self) -> i32 {
        self.source_id
    }

    /// Update the hash of the image's URI.
    pub fn update_image_uri_hash(&self) -> Result<(), DataError> {
        use crate::schema::shows::dsl::*;
        let db = connection();
        let con = db.get()?;

        info!("Updating the hash for image URI for podcast {}", self.title);
        diesel::update(shows.filter(id.eq(self.source_id)))
            .set(image_uri_hash.eq(calculate_hash(&self.image_uri)))
            .execute(&con)
            .map(|_| ())
            .map_err(From::from)
    }

    /// Update the timestamp when the image has been cached.
    pub fn update_image_cached(&self) -> Result<(), DataError> {
        use crate::schema::shows::dsl::*;
        let db = connection();
        let con = db.get()?;

        info!(
            "Updating the timestamp for when the image was last downloaded for podcast {}",
            self.title
        );
        diesel::update(shows.filter(id.eq(self.source_id)))
            .set(image_cached.eq(Utc::now().naive_utc()))
            .execute(&con)
            .map(|_| ())
            .map_err(From::from)
    }

    /// Update the image's timestamp and URI hash value.
    pub fn update_image_cache_values(&self) -> Result<(), DataError> {
        match self.image_uri_hash() {
            None => self.update_image_uri_hash()?,
            Some(hash) => {
                if calculate_hash(&self.image_uri()) != hash {
                    self.update_image_uri_hash()?;
                }
            }
        }
        match self.update_image_cached() {
            Ok(s) => Ok(s),
            Err(e) => Err(e),
        }
    }
}

#[derive(Queryable, Debug, Clone)]
/// Diesel Model of the Show cover query.
/// Used for fetching information about a Show's cover.
pub struct ShowCoverModel {
    id: i32,
    title: String,
    image_uri: Option<String>,
    image_uri_hash: Option<i64>,
    image_cached: NaiveDateTime,
}

impl From<Show> for ShowCoverModel {
    fn from(p: Show) -> ShowCoverModel {
        ShowCoverModel {
            id: p.id(),
            title: p.title,
            image_uri: p.image_uri,
            image_uri_hash: p.image_uri_hash,
            image_cached: p.image_cached,
        }
    }
}

impl ShowCoverModel {
    /// Get the Feed `id`.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the Feed `title`.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the `image_uri`.
    ///
    /// Represents the uri(url usually) that the Feed cover image is located at.
    pub fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_ref().map(|s| s.as_str())
    }

    /// Get the `image_uri_hash`.
    pub fn image_uri_hash(&self) -> Option<i64> {
        self.image_uri_hash
    }

    /// Get the `image_cached`.
    pub fn image_cached(&self) -> &NaiveDateTime {
        &self.image_cached
    }

    /// Determine whether a cached image is valid.
    ///
    /// A cached image is valid for a maximum of 4 weeks from the time of its previous download.
    /// Otherwise, a cached image is only valid so long as the hash of its URI remains unchanged.
    pub fn is_cached_image_valid(&self) -> bool {
        let cache_valid_duration = Duration::weeks(4);
        if Utc::now()
            .naive_utc()
            .signed_duration_since(*self.image_cached())
            > cache_valid_duration
        {
            return false;
        }
        if let Some(new) = &self.image_uri() {
            if let Some(orig) = self.image_uri_hash() {
                return calculate_hash(new) == orig;
            }
        }
        false
    }
}
