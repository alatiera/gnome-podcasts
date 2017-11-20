use diesel::prelude::*;
use diesel;

use schema::{episode, podcast, source};
use models::{Podcast, Source};
use connection;
use errors::*;

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

    fn index(&self) {
        use schema::source::dsl::*;

        let tempdb = connection().get().unwrap();
        // Throw away the result like `insert or ignore`
        // Diesel deos not support `insert or ignore` yet.
        let _ = diesel::insert_into(source).values(self).execute(&*tempdb);
    }

    // Look out for when tryinto lands into stable.
    pub fn into_source(self) -> QueryResult<Source> {
        self.index();

        dbqueries::get_source_from_uri(self.uri)
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

impl<'a> NewEpisode<'a> {
    // TODO: Currently using diesel from master git.
    // Watch out for v0.99.0 beta and change the toml.
    // TODO: Refactor into batch indexes instead.
    pub fn index(&self, con: &SqliteConnection) -> QueryResult<()> {
        use schema::episode::dsl::*;

        let ep = {
            // let tempdb = db.lock().unwrap();
            // dbqueries::get_episode_from_uri(&tempdb, self.uri.unwrap())
            dbqueries::get_episode_from_uri(self.uri.unwrap())
        };

        match ep {
            Ok(foo) => if foo.title() != self.title
                || foo.published_date() != self.published_date.as_ref().map(|x| x.as_str())
            {
                // let tempdb = db.lock().unwrap();
                diesel::replace_into(episode).values(self).execute(con)?;
                // .execute(&*tempdb)?;
            },
            Err(_) => {
                // let tempdb = db.lock().unwrap();
                diesel::insert_into(episode).values(self).execute(con)?;
                // .execute(&*tempdb)?;
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
        Ok(dbqueries::get_podcast_from_title(&self.title)?)
    }

    fn index(&self) -> QueryResult<()> {
        use schema::podcast::dsl::*;
        let pd = dbqueries::get_podcast_from_title(&self.title);

        match pd {
            Ok(foo) => if foo.link() != self.link {
                let tempdb = connection().get().unwrap();
                diesel::replace_into(podcast)
                    .values(self)
                    .execute(&*tempdb)?;
            },
            Err(_) => {
                let tempdb = connection().get().unwrap();
                diesel::insert_into(podcast).values(self).execute(&*tempdb)?;
            }
        }
        Ok(())
    }
}
