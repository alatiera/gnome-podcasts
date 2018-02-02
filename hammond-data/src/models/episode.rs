use chrono::prelude::*;
use diesel;
use diesel::SaveChangesDsl;
use diesel::prelude::*;

use database::connection;
use errors::*;
use models::{Podcast, Save};
use schema::episode;

#[derive(Queryable, Identifiable, AsChangeset, Associations, PartialEq)]
#[table_name = "episode"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, podcast_id)]
#[belongs_to(Podcast, foreign_key = "podcast_id")]
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
    favorite: bool,
    archive: bool,
    podcast_id: i32,
}

impl Save<Episode> for Episode {
    /// Helper method to easily save/"sync" current state of self to the Database.
    fn save(&self) -> Result<Episode> {
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

    /// Set the `title`.
    pub fn set_title(&mut self, value: &str) {
        self.title = value.to_string();
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(|s| s.as_str())
    }

    /// Set the `uri`.
    pub fn set_uri(&mut self, value: Option<&str>) {
        self.uri = value.map(|x| x.to_string());
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

    /// Get the `description`.
    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_str())
    }

    /// Set the `description`.
    pub fn set_description(&mut self, value: Option<&str>) {
        self.description = value.map(|x| x.to_string());
    }

    /// Get the Episode's `guid`.
    pub fn guid(&self) -> Option<&str> {
        self.guid.as_ref().map(|s| s.as_str())
    }

    /// Set the `guid`.
    pub fn set_guid(&mut self, value: Option<&str>) {
        self.guid = value.map(|x| x.to_string());
    }

    /// Get the `epoch` value.
    ///
    /// Retrieved from the rss Item publish date.
    /// Value is set to Utc whenever possible.
    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    /// Set the `epoch`.
    pub fn set_epoch(&mut self, value: i32) {
        self.epoch = value;
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

    /// Set the `duration`.
    pub fn set_duration(&mut self, value: Option<i32>) {
        self.duration = value;
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

    /// Represents the archiving policy for the episode.
    pub fn archive(&self) -> bool {
        self.archive
    }

    /// Set the `archive` policy.
    ///
    /// If true, the download cleanr will ignore the episode
    /// and the corresponding media value will never be automaticly deleted.
    pub fn set_archive(&mut self, b: bool) {
        self.archive = b
    }

    /// Get the `favorite` status of the `Episode`.
    pub fn favorite(&self) -> bool {
        self.favorite
    }

    /// Set `favorite` status.
    pub fn set_favorite(&mut self, b: bool) {
        self.favorite = b
    }

    /// `Podcast` table foreign key.
    pub fn podcast_id(&self) -> i32 {
        self.podcast_id
    }

    /// Sets the `played` value with the current `epoch` timestap and save it.
    pub fn set_played_now(&mut self) -> Result<()> {
        let epoch = Utc::now().timestamp() as i32;
        self.set_played(Some(epoch));
        self.save().map(|_| ())
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[table_name = "episode"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, podcast_id)]
#[derive(Debug, Clone)]
/// Diesel Model to be used for constructing `EpisodeWidgets`.
pub struct EpisodeWidgetQuery {
    rowid: i32,
    title: String,
    uri: Option<String>,
    local_uri: Option<String>,
    epoch: i32,
    length: Option<i32>,
    duration: Option<i32>,
    played: Option<i32>,
    // favorite: bool,
    // archive: bool,
    podcast_id: i32,
}

impl From<Episode> for EpisodeWidgetQuery {
    fn from(e: Episode) -> EpisodeWidgetQuery {
        EpisodeWidgetQuery {
            rowid: e.rowid,
            title: e.title,
            uri: e.uri,
            local_uri: e.local_uri,
            epoch: e.epoch,
            length: e.length,
            duration: e.duration,
            played: e.played,
            podcast_id: e.podcast_id,
        }
    }
}

impl Save<usize> for EpisodeWidgetQuery {
    /// Helper method to easily save/"sync" current state of self to the Database.
    fn save(&self) -> Result<usize> {
        use schema::episode::dsl::*;

        let db = connection();
        let tempdb = db.get()?;

        diesel::update(episode.filter(rowid.eq(self.rowid)))
            .set(self)
            .execute(&*tempdb)
            .map_err(From::from)
    }
}

impl EpisodeWidgetQuery {
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

    /// Set the `duration`.
    pub fn set_duration(&mut self, value: Option<i32>) {
        self.duration = value;
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

    // /// Represents the archiving policy for the episode.
    // pub fn archive(&self) -> bool {
    //     self.archive
    // }

    // /// Set the `archive` policy.
    // ///
    // /// If true, the download cleanr will ignore the episode
    // /// and the corresponding media value will never be automaticly deleted.
    // pub fn set_archive(&mut self, b: bool) {
    //     self.archive = b
    // }

    // /// Get the `favorite` status of the `Episode`.
    // pub fn favorite(&self) -> bool {
    //     self.favorite
    // }

    // /// Set `favorite` status.
    // pub fn set_favorite(&mut self, b: bool) {
    //     self.favorite = b
    // }

    /// `Podcast` table foreign key.
    pub fn podcast_id(&self) -> i32 {
        self.podcast_id
    }

    /// Sets the `played` value with the current `epoch` timestap and save it.
    pub fn set_played_now(&mut self) -> Result<()> {
        let epoch = Utc::now().timestamp() as i32;
        self.set_played(Some(epoch));
        self.save().map(|_| ())
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[table_name = "episode"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, podcast_id)]
#[derive(Debug, Clone)]
/// Diesel Model to be used internal with the `utils::checkup` function.
pub struct EpisodeCleanerQuery {
    rowid: i32,
    local_uri: Option<String>,
    played: Option<i32>,
}

impl Save<usize> for EpisodeCleanerQuery {
    /// Helper method to easily save/"sync" current state of self to the Database.
    fn save(&self) -> Result<usize> {
        use schema::episode::dsl::*;

        let db = connection();
        let tempdb = db.get()?;

        diesel::update(episode.filter(rowid.eq(self.rowid)))
            .set(self)
            .execute(&*tempdb)
            .map_err(From::from)
    }
}

impl From<Episode> for EpisodeCleanerQuery {
    fn from(e: Episode) -> EpisodeCleanerQuery {
        EpisodeCleanerQuery {
            rowid: e.rowid(),
            local_uri: e.local_uri,
            played: e.played,
        }
    }
}

impl EpisodeCleanerQuery {
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
#[table_name = "episode"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, podcast_id)]
#[derive(Debug, Clone)]
/// Diesel Model to be used for FIXME.
pub struct EpisodeMinimal {
    rowid: i32,
    title: String,
    uri: Option<String>,
    epoch: i32,
    duration: Option<i32>,
    guid: Option<String>,
    podcast_id: i32,
}

impl From<Episode> for EpisodeMinimal {
    fn from(e: Episode) -> Self {
        EpisodeMinimal {
            rowid: e.rowid,
            title: e.title,
            uri: e.uri,
            guid: e.guid,
            epoch: e.epoch,
            duration: e.duration,
            podcast_id: e.podcast_id,
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

    /// Get the `duration` value.
    ///
    /// The number represents the duration of the item/episode in seconds.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// `Podcast` table foreign key.
    pub fn podcast_id(&self) -> i32 {
        self.podcast_id
    }
}
