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
use chrono::{NaiveDateTime, Utc};
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

    /// Get the Feed `image_cached`.
    pub fn image_cached(&self) -> &NaiveDateTime {
        &self.image_cached
    }

    /// `Source` table foreign key.
    pub fn source_id(&self) -> i32 {
        self.source_id
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
}

#[derive(Queryable, Debug, Clone)]
/// Diesel Model of the Show cover query.
/// Used for fetching information about a Show's cover.
pub struct ShowCoverModel {
    id: i32,
    title: String,
    image_uri: Option<String>,
    image_cached: NaiveDateTime,
}

impl From<Show> for ShowCoverModel {
    fn from(p: Show) -> ShowCoverModel {
        ShowCoverModel {
            id: p.id(),
            title: p.title,
            image_uri: p.image_uri,
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

    /// Get the Feed `image_cached`.
    pub fn image_cached(&self) -> &NaiveDateTime {
        &self.image_cached
    }
}
