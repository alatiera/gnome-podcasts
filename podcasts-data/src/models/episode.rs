// episode.rs
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

use chrono::prelude::*;
use diesel;
use diesel::prelude::*;
use diesel::SaveChangesDsl;

use database::connection;
use errors::DataError;
use models::{Save, Show};
use schema::episodes;

#[derive(Queryable, Identifiable, AsChangeset, Associations, PartialEq)]
#[table_name = "episodes"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, show_id)]
#[belongs_to(Show, foreign_key = "show_id")]
#[derive(Debug, Clone)]
/// Diesel Model of the episode table.
pub struct Episode {
    rowid: i32,
    title: String,
    uri: Option<String>,
    local_uri: Option<String>,
    description: Option<String>,
    epoch: i32,
    length: Option<i32>,
    duration: Option<i32>,
    guid: Option<String>,
    played: Option<i32>,
    show_id: i32,
}

impl Save<Episode> for Episode {
    type Error = DataError;

    /// Helper method to easily save/"sync" current state of self to the
    /// Database.
    fn save(&self) -> Result<Episode, Self::Error> {
        let db = connection();
        let tempdb = db.get()?;

        self.save_changes::<Episode>(&*tempdb).map_err(From::from)
    }
}

impl Episode {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `title` field.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(|s| s.as_str())
    }

    /// Get the value of the `local_uri`.
    ///
    /// Represents the local uri,usually filesystem path,
    /// that the media file will be located at.
    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_ref().map(|s| s.as_str())
    }

    /// Get the `description`.
    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_str())
    }

    /// Get the Episode's `guid`.
    pub fn guid(&self) -> Option<&str> {
        self.guid.as_ref().map(|s| s.as_str())
    }

    /// Get the `epoch` value.
    ///
    /// Retrieved from the rss Item publish date.
    /// Value is set to Utc whenever possible.
    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    /// Get the `length`.
    ///
    /// The number represents the size of the file in bytes.
    pub fn length(&self) -> Option<i32> {
        self.length
    }

    /// Get the `duration` value.
    ///
    /// The number represents the duration of the item/episode in seconds.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// Epoch representation of the last time the episode was played.
    ///
    /// None/Null for unplayed.
    pub fn played(&self) -> Option<i32> {
        self.played
    }

    /// `Show` table foreign key.
    pub fn show_id(&self) -> i32 {
        self.show_id
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[table_name = "episodes"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, show_id)]
#[derive(Debug, Clone)]
/// Diesel Model to be used for constructing `EpisodeWidgets`.
pub struct EpisodeWidgetModel {
    rowid: i32,
    title: String,
    uri: Option<String>,
    local_uri: Option<String>,
    epoch: i32,
    length: Option<i32>,
    duration: Option<i32>,
    played: Option<i32>,
    show_id: i32,
}

impl From<Episode> for EpisodeWidgetModel {
    fn from(e: Episode) -> EpisodeWidgetModel {
        EpisodeWidgetModel {
            rowid: e.rowid,
            title: e.title,
            uri: e.uri,
            local_uri: e.local_uri,
            epoch: e.epoch,
            length: e.length,
            duration: e.duration,
            played: e.played,
            show_id: e.show_id,
        }
    }
}

impl Save<usize> for EpisodeWidgetModel {
    type Error = DataError;

    /// Helper method to easily save/"sync" current state of self to the
    /// Database.
    fn save(&self) -> Result<usize, Self::Error> {
        use schema::episodes::dsl::*;

        let db = connection();
        let tempdb = db.get()?;

        diesel::update(episodes.filter(rowid.eq(self.rowid)))
            .set(self)
            .execute(&*tempdb)
            .map_err(From::from)
    }
}

impl EpisodeWidgetModel {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `title` field.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(|s| s.as_str())
    }

    /// Get the value of the `local_uri`.
    ///
    /// Represents the local uri,usually filesystem path,
    /// that the media file will be located at.
    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_ref().map(|s| s.as_str())
    }

    /// Set the `local_uri`.
    pub fn set_local_uri(&mut self, value: Option<&str>) {
        self.local_uri = value.map(|x| x.to_string());
    }

    /// Get the `epoch` value.
    ///
    /// Retrieved from the rss Item publish date.
    /// Value is set to Utc whenever possible.
    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    /// Get the `length`.
    ///
    /// The number represents the size of the file in bytes.
    pub fn length(&self) -> Option<i32> {
        self.length
    }

    /// Set the `length`.
    pub fn set_length(&mut self, value: Option<i32>) {
        self.length = value;
    }

    /// Get the `duration` value.
    ///
    /// The number represents the duration of the item/episode in seconds.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// Epoch representation of the last time the episode was played.
    ///
    /// None/Null for unplayed.
    pub fn played(&self) -> Option<i32> {
        self.played
    }

    /// Set the `played` value.
    fn set_played(&mut self, value: Option<i32>) {
        self.played = value;
    }

    /// `Show` table foreign key.
    pub fn show_id(&self) -> i32 {
        self.show_id
    }

    /// Sets the `played` value with the current `epoch` timestap and save it.
    pub fn set_played_now(&mut self) -> Result<(), DataError> {
        let epoch = Utc::now().timestamp() as i32;
        self.set_played(Some(epoch));
        self.save().map(|_| ())
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[table_name = "episodes"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, show_id)]
#[derive(Debug, Clone)]
/// Diesel Model to be used internal with the `utils::checkup` function.
pub struct EpisodeCleanerModel {
    rowid: i32,
    local_uri: Option<String>,
    played: Option<i32>,
}

impl Save<usize> for EpisodeCleanerModel {
    type Error = DataError;

    /// Helper method to easily save/"sync" current state of self to the
    /// Database.
    fn save(&self) -> Result<usize, Self::Error> {
        use schema::episodes::dsl::*;

        let db = connection();
        let tempdb = db.get()?;

        diesel::update(episodes.filter(rowid.eq(self.rowid)))
            .set(self)
            .execute(&*tempdb)
            .map_err(From::from)
    }
}

impl From<Episode> for EpisodeCleanerModel {
    fn from(e: Episode) -> EpisodeCleanerModel {
        EpisodeCleanerModel {
            rowid: e.rowid(),
            local_uri: e.local_uri,
            played: e.played,
        }
    }
}

impl EpisodeCleanerModel {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `local_uri`.
    ///
    /// Represents the local uri,usually filesystem path,
    /// that the media file will be located at.
    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_ref().map(|s| s.as_str())
    }

    /// Set the `local_uri`.
    pub fn set_local_uri(&mut self, value: Option<&str>) {
        self.local_uri = value.map(|x| x.to_string());
    }

    /// Epoch representation of the last time the episode was played.
    ///
    /// None/Null for unplayed.
    pub fn played(&self) -> Option<i32> {
        self.played
    }

    /// Set the `played` value.
    pub fn set_played(&mut self, value: Option<i32>) {
        self.played = value;
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[table_name = "episodes"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, show_id)]
#[derive(Debug, Clone)]
/// Diesel Model to be used for FIXME.
pub struct EpisodeMinimal {
    rowid: i32,
    title: String,
    uri: Option<String>,
    epoch: i32,
    length: Option<i32>,
    duration: Option<i32>,
    guid: Option<String>,
    show_id: i32,
}

impl From<Episode> for EpisodeMinimal {
    fn from(e: Episode) -> Self {
        EpisodeMinimal {
            rowid: e.rowid,
            title: e.title,
            uri: e.uri,
            length: e.length,
            guid: e.guid,
            epoch: e.epoch,
            duration: e.duration,
            show_id: e.show_id,
        }
    }
}

impl EpisodeMinimal {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `title` field.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(|s| s.as_str())
    }

    /// Get the Episode's `guid`.
    pub fn guid(&self) -> Option<&str> {
        self.guid.as_ref().map(|s| s.as_str())
    }

    /// Get the `epoch` value.
    ///
    /// Retrieved from the rss Item publish date.
    /// Value is set to Utc whenever possible.
    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    /// Get the `length`.
    ///
    /// The number represents the size of the file in bytes.
    pub fn length(&self) -> Option<i32> {
        self.length
    }

    /// Get the `duration` value.
    ///
    /// The number represents the duration of the item/episode in seconds.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// `Show` table foreign key.
    pub fn show_id(&self) -> i32 {
        self.show_id
    }
}
