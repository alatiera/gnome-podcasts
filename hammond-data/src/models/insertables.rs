use diesel::prelude::*;

use schema::{episode, podcast, source};
use models::{Episode, Podcast, Source};
use utils::url_cleaner;
use errors::*;

use dbqueries;
use database::connection;

#[derive(Insertable)]
#[table_name = "source"]
#[derive(Debug, Clone)]
pub struct NewSource {
    uri: String,
    last_modified: Option<String>,
    http_etag: Option<String>,
}

impl NewSource {
    pub fn new_with_uri(uri: &str) -> NewSource {
        let uri = url_cleaner(uri);
        NewSource {
            uri,
            last_modified: None,
            http_etag: None,
        }
    }

    fn index(&self) -> Result<()> {
        let db = connection();
        let con = db.get()?;

        // Throw away the result like `insert or ignore`
        // Diesel deos not support `insert or ignore` yet.
        let _ = dbqueries::insert_new_source(&con, self);
        Ok(())
    }

    // Look out for when tryinto lands into stable.
    pub fn into_source(self) -> Result<Source> {
        self.index()?;
        dbqueries::get_source_from_uri(&self.uri)
    }
}

#[derive(Insertable)]
#[table_name = "episode"]
#[derive(Debug, Clone, Default)]
pub struct NewEpisode {
    pub title: Option<String>,
    pub uri: Option<String>,
    pub description: Option<String>,
    pub published_date: Option<String>,
    pub length: Option<i32>,
    pub guid: Option<String>,
    pub epoch: i32,
    pub podcast_id: i32,
}

impl NewEpisode {
    // TODO: Currently using diesel from master git.
    // Watch out for v0.99.0 beta and change the toml.
    // TODO: Refactor into batch indexes instead.
    pub fn into_episode(self, con: &SqliteConnection) -> Result<Episode> {
        self.index(con)?;
        Ok(dbqueries::get_episode_from_uri(con, &self.uri.unwrap())?)
    }

    pub fn index(&self, con: &SqliteConnection) -> QueryResult<()> {
        let ep = dbqueries::get_episode_from_uri(con, &self.uri.clone().unwrap());

        match ep {
            Ok(foo) => {
                if foo.podcast_id() != self.podcast_id {
                    error!("NEP pid: {}, EP pid: {}", self.podcast_id, foo.podcast_id());
                };

                if foo.title() != self.title.as_ref().map(|x| x.as_str())
                    || foo.published_date() != self.published_date.as_ref().map(|x| x.as_str())
                {
                    dbqueries::replace_episode(con, self)?;
                }
            }
            Err(_) => {
                dbqueries::insert_new_episode(con, self)?;
            }
        }
        Ok(())
    }
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
    pub fn into_podcast(self) -> Result<Podcast> {
        self.index()?;
        Ok(dbqueries::get_podcast_from_source_id(self.source_id)?)
    }

    pub fn index(&self) -> Result<()> {
        let pd = dbqueries::get_podcast_from_source_id(self.source_id);

        let db = connection();
        let con = db.get()?;
        match pd {
            Ok(foo) => {
                if foo.source_id() != self.source_id {
                    error!("NPD sid: {}, PD sid: {}", self.source_id, foo.source_id());
                };

                if (foo.link() != self.link) || (foo.title() != self.title)
                    || (foo.image_uri() != self.image_uri.as_ref().map(|x| x.as_str()))
                {
                    dbqueries::replace_podcast(&con, self)?;
                }
            }
            Err(_) => {
                dbqueries::insert_new_podcast(&con, self)?;
            }
        }
        Ok(())
    }
}
