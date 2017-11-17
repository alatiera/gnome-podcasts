use diesel::prelude::*;

use schema::{episode, podcast, source};
use models::{Podcast, Source};
use index_feed::Database;
use errors::*;

use index_feed;
use dbqueries;

#[derive(Insertable)]
#[table_name = "source"]
#[derive(Debug, Clone)]
pub struct NewSource<'a> {
    uri: &'a str,
    last_modified: Option<&'a str>,
    http_etag: Option<&'a str>,
}

impl<'a> NewSource<'a> {
    pub fn new_with_uri(uri: &'a str) -> NewSource {
        NewSource {
            uri,
            last_modified: None,
            http_etag: None,
        }
    }

    // Look out for when tryinto lands into stable.
    pub fn into_source(self, db: &Database) -> QueryResult<Source> {
        let tempdb = db.lock().unwrap();
        index_feed::index_source(&tempdb, &self);
        dbqueries::get_source_from_uri(&tempdb, self.uri)
    }
}

#[derive(Insertable)]
#[table_name = "episode"]
#[derive(Debug, Clone)]
pub struct NewEpisode<'a> {
    pub title: Option<&'a str>,
    pub uri: Option<&'a str>,
    pub description: Option<&'a str>,
    pub published_date: Option<String>,
    pub length: Option<i32>,
    pub guid: Option<&'a str>,
    pub epoch: i32,
    pub podcast_id: i32,
}

#[derive(Insertable)]
#[table_name = "podcast"]
#[derive(Debug, Clone)]
pub struct NewPodcast {
    pub title: String,
    pub link: String,
    pub description: String,
    pub image_uri: Option<String>,
    pub source_id: i32,
}

impl NewPodcast {
    // Look out for when tryinto lands into stable.
    pub fn into_podcast(self, db: &Database) -> Result<Podcast> {
        let tempdb = db.lock().unwrap();
        index_feed::index_podcast(&tempdb, &self)?;

        Ok(dbqueries::get_podcast_from_title(&tempdb, &self.title)?)
    }
}
