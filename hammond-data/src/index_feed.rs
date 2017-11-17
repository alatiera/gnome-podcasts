use diesel::prelude::*;
use diesel;
use rss;
use rayon::prelude::*;

use dbqueries;
use models::*;
use errors::*;
use feedparser;

use std::sync::{Arc, Mutex};

pub type Database = Arc<Mutex<SqliteConnection>>;

#[derive(Debug)]
pub struct Feed(rss::Channel, Source);

impl Feed {
    pub fn new_from_source(db: &Database, s: Source) -> Result<Feed> {
        s.refresh(db)
    }

    pub fn new_from_channel_source(chan: rss::Channel, s: Source) -> Feed {
        Feed(chan, s)
    }

    fn index(&self, db: &Database) -> Result<()> {
        let tempdb = db.lock().unwrap();
        let pd = self.index_channel(&tempdb)?;
        drop(tempdb);

        self.index_channel_items(db, &pd)?;
        Ok(())
    }

    fn index_channel(&self, con: &SqliteConnection) -> Result<Podcast> {
        let pd = feedparser::parse_podcast(&self.0, self.1.id());
        // Convert NewPodcast to Podcast
        insert_return_podcast(con, &pd)
    }

    fn index_channel_items(&self, db: &Database, pd: &Podcast) -> Result<()> {
        let it = self.0.items();
        let episodes: Vec<_> = it.par_iter()
            .map(|x| feedparser::parse_episode(x, pd.id()))
            .collect();

        let conn = db.lock().unwrap();
        let e = conn.transaction::<(), Error, _>(|| {
            episodes.iter().for_each(|x| {
                let e = index_episode(&conn, x);
                if let Err(err) = e {
                    error!("Failed to index episode: {:?}.", x);
                    error!("Error msg: {}", err);
                };
            });
            Ok(())
        });
        drop(conn);

        e
    }
}

pub fn index_source(con: &SqliteConnection, foo: &NewSource) {
    use schema::source::dsl::*;

    // Throw away the result like `insert or ignore`
    // Diesel deos not support `insert or ignore` yet.
    let _ = diesel::insert_into(source).values(foo).execute(con);
}

fn index_podcast(con: &SqliteConnection, pd: &NewPodcast) -> Result<()> {
    use schema::podcast::dsl::*;

    match dbqueries::get_podcast_from_title(con, &pd.title) {
        Ok(foo) => if foo.link() != pd.link || foo.description() != pd.description {
            diesel::replace_into(podcast).values(pd).execute(con)?;
        },
        Err(_) => {
            diesel::insert_into(podcast).values(pd).execute(con)?;
        }
    }
    Ok(())
}

// TODO: Currently using diesel from master git.
// Watch out for v0.99.0 beta and change the toml.
fn index_episode(con: &SqliteConnection, ep: &NewEpisode) -> QueryResult<()> {
    use schema::episode::dsl::*;

    match dbqueries::get_episode_from_uri(con, ep.uri.unwrap()) {
        Ok(foo) => if foo.title() != ep.title
            || foo.published_date() != ep.published_date.as_ref().map(|x| x.as_str())
        {
            diesel::replace_into(episode).values(ep).execute(con)?;
        },
        Err(_) => {
            diesel::insert_into(episode).values(ep).execute(con)?;
        }
    }
    Ok(())
}

pub fn insert_return_source(con: &SqliteConnection, url: &str) -> Result<Source> {
    let foo = NewSource::new_with_uri(url);
    index_source(con, &foo);

    Ok(dbqueries::get_source_from_uri(con, foo.uri)?)
}

fn insert_return_podcast(con: &SqliteConnection, pd: &NewPodcast) -> Result<Podcast> {
    index_podcast(con, pd)?;

    Ok(dbqueries::get_podcast_from_title(con, &pd.title)?)
}

// fn insert_return_episode(con: &SqliteConnection, ep: &NewEpisode) -> Result<Episode> {
//     index_episode(con, ep)?;

//     Ok(dbqueries::get_episode_from_uri(con, ep.uri.unwrap())?)
// }

pub fn full_index_loop(db: &Database) -> Result<()> {
    let mut f = fetch_all_feeds(db)?;

    index_feeds(db, &mut f);
    info!("Indexing done.");
    Ok(())
}

pub fn index_feeds(db: &Database, f: &mut [Feed]) {
    f.into_par_iter().for_each(|x| {
        let e = x.index(db);
        if e.is_err() {
            error!("Error While trying to update the database.");
            error!("Error msg: {}", e.unwrap_err());
        };
    });
}

pub fn fetch_all_feeds(db: &Database) -> Result<Vec<Feed>> {
    let feeds = {
        let conn = db.lock().unwrap();
        dbqueries::get_sources(&conn)?
    };

    let results = fetch_feeds(db, feeds);
    Ok(results)
}

pub fn fetch_feeds(db: &Database, feeds: Vec<Source>) -> Vec<Feed> {
    let results: Vec<_> = feeds
        .into_par_iter()
        .filter_map(|x| {
            let uri = x.uri().to_owned();
            let l = Feed::new_from_source(db, x);
            if l.is_ok() {
                l.ok()
            } else {
                error!("Error While trying to fetch from source: {}.", uri);
                error!("Error msg: {}", l.unwrap_err());
                None
            }
        })
        .collect();

    results
}

#[cfg(test)]
mod tests {

    extern crate rand;
    extern crate tempdir;

    use diesel::prelude::*;
    use rss;
    use self::rand::Rng;

    use std::io::BufReader;
    use std::path::PathBuf;
    use std::fs;

    use super::*;

    struct TempDB(tempdir::TempDir, PathBuf, SqliteConnection);

    /// Create and return a Temporary DB.
    /// Will be destroed once the returned variable(s) is dropped.
    fn get_temp_db() -> TempDB {
        let mut rng = rand::thread_rng();

        let tmp_dir = tempdir::TempDir::new("hammond_unit_test").unwrap();
        let db_path = tmp_dir
            .path()
            .join(format!("hammonddb_{}.db", rng.gen::<usize>()));

        let db = SqliteConnection::establish(db_path.to_str().unwrap()).unwrap();
        ::run_migration_on(&db).unwrap();

        TempDB(tmp_dir, db_path, db)
    }

    #[test]
    /// Insert feeds and update/index them.
    fn test_index_loop() {
        let TempDB(_tmp_dir, _db_path, db) = get_temp_db();
        let db = Arc::new(Mutex::new(db));

        let inpt = vec![
            "https://request-for-explanation.github.io/podcast/rss.xml",
            "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
            "http://feeds.propublica.org/propublica/podcast",
            "http://feeds.feedburner.com/linuxunplugged",
        ];

        inpt.iter().for_each(|feed| {
            let tempdb = db.lock().unwrap();
            index_source(&tempdb, &NewSource::new_with_uri(feed));
        });

        full_index_loop(&db).unwrap();

        // Run again to cover Unique constrains erros.
        full_index_loop(&db).unwrap();
    }

    #[test]
    fn test_complete_index() {
        let TempDB(_tmp_dir, _db_path, db) = get_temp_db();
        // complete_index runs in parallel so it requires a mutex as argument.
        let m = Arc::new(Mutex::new(db));

        // vec of (path, url) tuples.
        let urls = vec![
            (
                "tests/feeds/Intercepted.xml",
                "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
            ),
            (
                "tests/feeds/LinuxUnplugged.xml",
                "http://feeds.feedburner.com/linuxunplugged",
            ),
            (
                "tests/feeds/TheBreakthrough.xml",
                "http://feeds.feedburner.com/propublica/podcast",
            ),
            (
                "tests/feeds/R4Explanation.xml",
                "https://request-for-explanation.github.io/podcast/rss.xml",
            ),
        ];

        let mut feeds: Vec<_> = urls.iter()
            .map(|&(path, url)| {
                let tempdb = m.lock().unwrap();
                // Create and insert a Source into db
                let s = insert_return_source(&tempdb, url).unwrap();
                drop(tempdb);

                // open the xml file
                let feed = fs::File::open(path).unwrap();
                // parse it into a channel
                let chan = rss::Channel::read_from(BufReader::new(feed)).unwrap();
                Feed::new_from_channel_source(chan, s)
            })
            .collect();

        // Index the channel
        index_feeds(&m, &mut feeds);

        // Assert the index rows equal the controlled results
        let tempdb = m.lock().unwrap();
        assert_eq!(dbqueries::get_sources(&tempdb).unwrap().len(), 4);
        assert_eq!(dbqueries::get_podcasts(&tempdb).unwrap().len(), 4);
        assert_eq!(dbqueries::get_episodes(&tempdb).unwrap().len(), 274);
    }
}
