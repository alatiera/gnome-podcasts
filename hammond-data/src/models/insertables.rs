use diesel::prelude::*;

use schema::{episode, podcast, source};
use models::queryables::{Episode, Podcast, Source};

use utils::url_cleaner;
use errors::*;

use dbqueries;
use diesel;
use database::connection;

trait Insert {
    fn insert(&self, &SqliteConnection) -> QueryResult<usize>;
}

trait Update {
    fn update(&self, &SqliteConnection, i32) -> QueryResult<usize>;
}

#[derive(Insertable)]
#[table_name = "source"]
#[derive(Debug, Clone)]
pub(crate) struct NewSource {
    uri: String,
    last_modified: Option<String>,
    http_etag: Option<String>,
}

impl Insert for NewSource {
    fn insert(&self, con: &SqliteConnection) -> QueryResult<usize> {
        use schema::source::dsl::*;
        diesel::insert_into(source).values(self).execute(&*con)
    }
}

impl NewSource {
    pub(crate) fn new_with_uri(uri: &str) -> NewSource {
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
        let _ = self.insert(&con);
        Ok(())
    }

    // Look out for when tryinto lands into stable.
    pub(crate) fn into_source(self) -> Result<Source> {
        self.index()?;
        dbqueries::get_source_from_uri(&self.uri)
    }
}

#[derive(Insertable, AsChangeset)]
#[table_name = "podcast"]
#[derive(Debug, Clone)]
pub(crate) struct NewPodcast {
    title: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    favorite: bool,
    archive: bool,
    always_dl: bool,
    source_id: i32,
}

impl Insert for NewPodcast {
    fn insert(&self, con: &SqliteConnection) -> QueryResult<usize> {
        use schema::podcast::dsl::*;
        diesel::insert_into(podcast).values(self).execute(&*con)
    }
}

impl Update for NewPodcast {
    fn update(&self, con: &SqliteConnection, podcast_id: i32) -> QueryResult<usize> {
        use schema::podcast::dsl::*;

        diesel::update(podcast.filter(id.eq(podcast_id)))
            .set(self)
            .execute(&*con)
    }
}

impl NewPodcast {
    // Look out for when tryinto lands into stable.
    pub(crate) fn into_podcast(self) -> Result<Podcast> {
        self.index()?;
        Ok(dbqueries::get_podcast_from_source_id(self.source_id)?)
    }

    pub(crate) fn index(&self) -> Result<()> {
        let pd = dbqueries::get_podcast_from_source_id(self.source_id);

        let db = connection();
        let con = db.get()?;
        match pd {
            Ok(foo) => {
                if foo.source_id() != self.source_id {
                    error!("NSPD sid: {}, SPD sid: {}", self.source_id, foo.source_id());
                };

                if (foo.link() != self.link) || (foo.title() != self.title)
                    || (foo.image_uri() != self.image_uri.as_ref().map(|x| x.as_str()))
                {
                    self.update(&con, *foo.id())?;
                }
            }
            Err(_) => {
                self.insert(&con)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(crate) struct NewPodcastBuilder {
    title: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    favorite: bool,
    archive: bool,
    always_dl: bool,
    source_id: i32,
}

#[allow(dead_code)]
impl NewPodcastBuilder {
    pub(crate) fn new() -> NewPodcastBuilder {
        NewPodcastBuilder::default()
    }

    pub(crate) fn title(mut self, s: String) -> NewPodcastBuilder {
        self.title = s;
        self
    }

    pub(crate) fn link(mut self, s: String) -> NewPodcastBuilder {
        self.link = s;
        self
    }

    pub(crate) fn description(mut self, s: String) -> NewPodcastBuilder {
        self.description = s;
        self
    }

    pub(crate) fn image_uri(mut self, s: Option<String>) -> NewPodcastBuilder {
        self.image_uri = s;
        self
    }

    pub(crate) fn source_id(mut self, s: i32) -> NewPodcastBuilder {
        self.source_id = s;
        self
    }

    pub(crate) fn favorite(mut self, s: bool) -> NewPodcastBuilder {
        self.favorite = s;
        self
    }

    pub(crate) fn archive(mut self, s: bool) -> NewPodcastBuilder {
        self.archive = s;
        self
    }

    pub(crate) fn always_dl(mut self, s: bool) -> NewPodcastBuilder {
        self.always_dl = s;
        self
    }

    pub(crate) fn build(self) -> NewPodcast {
        NewPodcast {
            title: self.title,
            link: self.link,
            description: self.description,
            image_uri: self.image_uri,
            favorite: self.favorite,
            archive: self.archive,
            always_dl: self.always_dl,
            source_id: self.source_id,
        }
    }
}

#[allow(dead_code)]
// Ignore the following geters. They are used in unit tests mainly.
impl NewPodcast {
    pub(crate) fn source_id(&self) -> i32 {
        self.source_id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn link(&self) -> &str {
        &self.link
    }

    pub(crate) fn description(&self) -> &str {
        &self.description
    }

    pub(crate) fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_ref().map(|s| s.as_str())
    }
}

#[derive(Insertable, AsChangeset)]
#[table_name = "episode"]
#[derive(Debug, Clone, Default)]
pub(crate) struct NewEpisode {
    title: Option<String>,
    uri: String,
    local_uri: Option<String>,
    description: Option<String>,
    published_date: Option<String>,
    length: Option<i32>,
    guid: Option<String>,
    epoch: i32,
    played: Option<i32>,
    favorite: bool,
    archive: bool,
    podcast_id: i32,
}

impl Insert for NewEpisode {
    fn insert(&self, con: &SqliteConnection) -> QueryResult<usize> {
        use schema::episode::dsl::*;
        diesel::insert_into(episode).values(self).execute(&*con)
    }
}

impl Update for NewEpisode {
    fn update(&self, con: &SqliteConnection, episode_id: i32) -> QueryResult<usize> {
        use schema::episode::dsl::*;

        diesel::update(episode.filter(id.eq(episode_id)))
            .set(self)
            .execute(&*con)
    }
}

impl NewEpisode {
    // TODO: Currently using diesel from master git.
    // Watch out for v0.99.0 beta and change the toml.
    // TODO: Refactor into batch indexes instead.
    pub(crate) fn into_episode(self, con: &SqliteConnection) -> Result<Episode> {
        self.index(con)?;
        Ok(dbqueries::get_episode_from_uri(con, &self.uri)?)
    }

    pub(crate) fn index(&self, con: &SqliteConnection) -> QueryResult<()> {
        let ep = dbqueries::get_episode_from_uri(con, &self.uri.clone());

        match ep {
            Ok(foo) => {
                if foo.podcast_id() != self.podcast_id {
                    error!("NEP pid: {}, EP pid: {}", self.podcast_id, foo.podcast_id());
                };

                if foo.title() != self.title.as_ref().map(|x| x.as_str())
                    || foo.published_date() != self.published_date.as_ref().map(|x| x.as_str())
                {
                    self.update(con, *foo.id())?;
                }
            }
            Err(_) => {
                self.insert(con)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(crate) struct NewEpisodeBuilder {
    title: Option<String>,
    uri: String,
    local_uri: Option<String>,
    description: Option<String>,
    published_date: Option<String>,
    length: Option<i32>,
    guid: Option<String>,
    epoch: i32,
    played: Option<i32>,
    favorite: bool,
    archive: bool,
    podcast_id: i32,
}

#[allow(dead_code)]
impl NewEpisodeBuilder {
    pub(crate) fn new() -> NewEpisodeBuilder {
        NewEpisodeBuilder::default()
    }

    pub(crate) fn title(mut self, s: Option<String>) -> NewEpisodeBuilder {
        self.title = s;
        self
    }

    pub(crate) fn uri(mut self, s: String) -> NewEpisodeBuilder {
        self.uri = s;
        self
    }

    pub(crate) fn local_uri(mut self, s: Option<String>) -> NewEpisodeBuilder {
        self.local_uri = s;
        self
    }

    pub(crate) fn description(mut self, s: Option<String>) -> NewEpisodeBuilder {
        self.description = s;
        self
    }

    pub(crate) fn published_date(mut self, s: Option<String>) -> NewEpisodeBuilder {
        self.published_date = s;
        self
    }

    pub(crate) fn length(mut self, s: Option<i32>) -> NewEpisodeBuilder {
        self.length = s;
        self
    }

    pub(crate) fn played(mut self, s: Option<i32>) -> NewEpisodeBuilder {
        self.played = s;
        self
    }

    pub(crate) fn guid(mut self, s: Option<String>) -> NewEpisodeBuilder {
        self.guid = s;
        self
    }

    pub(crate) fn epoch(mut self, s: i32) -> NewEpisodeBuilder {
        self.epoch = s;
        self
    }

    pub(crate) fn podcast_id(mut self, s: i32) -> NewEpisodeBuilder {
        self.podcast_id = s;
        self
    }

    pub(crate) fn favorite(mut self, s: bool) -> NewEpisodeBuilder {
        self.favorite = s;
        self
    }

    pub(crate) fn archive(mut self, s: bool) -> NewEpisodeBuilder {
        self.archive = s;
        self
    }

    pub(crate) fn build(self) -> NewEpisode {
        NewEpisode {
            title: self.title,
            uri: self.uri,
            local_uri: self.local_uri,
            description: self.description,
            published_date: self.published_date,
            length: self.length,
            guid: self.guid,
            epoch: self.epoch,
            played: self.played,
            favorite: self.favorite,
            archive: self.archive,
            podcast_id: self.podcast_id,
        }
    }
}

#[allow(dead_code)]
// Ignore the following geters. They are used in unit tests mainly.
impl NewEpisode {
    pub(crate) fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn uri(&self) -> &str {
        self.uri.as_ref()
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn published_date(&self) -> Option<&str> {
        self.published_date.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn guid(&self) -> Option<&str> {
        self.guid.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn epoch(&self) -> i32 {
        self.epoch
    }

    pub(crate) fn length(&self) -> Option<i32> {
        self.length
    }

    pub(crate) fn podcast_id(&self) -> i32 {
        self.podcast_id
    }
}
