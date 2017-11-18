use rayon::prelude::*;

use rss;

use dbqueries;
use parser;
use Database;

use models::{Podcast, Source};
use errors::*;

use std::sync::Arc;


#[derive(Debug)]
pub struct Feed {
    channel: rss::Channel,
    source: Source,
}

impl Feed {
    pub fn new_from_source(db: &Database, s: Source) -> Result<Feed> {
        s.refresh(db)
    }

    pub fn new_from_channel_source(chan: rss::Channel, s: Source) -> Feed {
        Feed {
            channel: chan,
            source: s,
        }
    }

    fn index(&self, db: &Database) -> Result<()> {
        let pd = self.index_channel(db)?;

        self.index_channel_items(db, &pd)?;
        Ok(())
    }

    fn index_channel(&self, db: &Database) -> Result<Podcast> {
        let pd = parser::new_podcast(&self.channel, self.source.id());
        // Convert NewPodcast to Podcast
        pd.into_podcast(db)
    }

    // TODO: Figure out transcactions.
    // The synchronous version where there was a db.lock() before the episodes.iter()
    // is actually faster.
    fn index_channel_items(&self, db: &Database, pd: &Podcast) -> Result<()> {
        let it = self.channel.items();
        let episodes: Vec<_> = it.par_iter()
            .map(|x| parser::new_episode(x, pd.id()))
            .collect();

        episodes.into_par_iter().for_each(|x| {
            let e = x.index(&Arc::clone(db));
            if let Err(err) = e {
                error!("Failed to index episode: {:?}.", x);
                error!("Error msg: {}", err);
            };
        });
        Ok(())
    }
}

pub fn index_all(db: &Database) -> Result<()> {
    let mut f = fetch_all(db)?;

    index(db, &mut f);
    info!("Indexing done.");
    Ok(())
}

pub fn index(db: &Database, f: &mut [Feed]) {
    f.into_par_iter().for_each(|x| {
        let e = x.index(&Arc::clone(db));
        if e.is_err() {
            error!("Error While trying to update the database.");
            error!("Error msg: {}", e.unwrap_err());
        };
    });
}

pub fn fetch_all(db: &Database) -> Result<Vec<Feed>> {
    let feeds = {
        let conn = db.lock().unwrap();
        dbqueries::get_sources(&conn)?
    };

    let results = fetch(db, feeds);
    Ok(results)
}

pub fn fetch(db: &Database, feeds: Vec<Source>) -> Vec<Feed> {
    let results: Vec<_> = feeds
        .into_par_iter()
        .filter_map(|x| {
            let uri = x.uri().to_owned();
            let l = Feed::new_from_source(&Arc::clone(db), x);
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
    use models::NewSource;

    use std::io::BufReader;
    use std::path::PathBuf;
    use std::fs;
    use std::sync::Mutex;

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
            NewSource::new_with_uri(feed)
                .into_source(&db.clone())
                .unwrap();
        });

        index_all(&db).unwrap();

        // Run again to cover Unique constrains erros.
        index_all(&db).unwrap();
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
                // Create and insert a Source into db
                let s = NewSource::new_with_uri(url)
                    .into_source(&m.clone())
                    .unwrap();

                // open the xml file
                let feed = fs::File::open(path).unwrap();
                // parse it into a channel
                let chan = rss::Channel::read_from(BufReader::new(feed)).unwrap();
                Feed::new_from_channel_source(chan, s)
            })
            .collect();

        // Index the channels
        index(&m, &mut feeds);

        // Assert the index rows equal the controlled results
        let tempdb = m.lock().unwrap();
        assert_eq!(dbqueries::get_sources(&tempdb).unwrap().len(), 4);
        assert_eq!(dbqueries::get_podcasts(&tempdb).unwrap().len(), 4);
        assert_eq!(dbqueries::get_episodes(&tempdb).unwrap().len(), 274);
    }
}
