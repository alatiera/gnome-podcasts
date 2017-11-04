#![allow(dead_code)]

use diesel::prelude::*;
use diesel;
use rss;
use reqwest;
use rayon::prelude::*;

use schema;
use dbqueries;
use models::*;
use errors::*;
use feedparser;

use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Feed(pub reqwest::Response, pub Source);

pub type Database = Arc<Mutex<SqliteConnection>>;

fn index_source(con: &SqliteConnection, foo: &NewSource) -> Result<()> {
    match dbqueries::load_source_from_uri(con, foo.uri) {
        Ok(_) => Ok(()),
        Err(_) => {
            diesel::insert(foo)
                .into(schema::source::table)
                .execute(con)?;
            Ok(())
        }
    }
}

fn index_podcast(con: &SqliteConnection, pd: &NewPodcast) -> Result<()> {
    match dbqueries::load_podcast_from_title(con, &pd.title) {
        Ok(mut foo) => if foo.link() != pd.link || foo.description() != pd.description {
            foo.set_link(&pd.link);
            foo.set_description(&pd.description);
            foo.set_image_uri(pd.image_uri.as_ref().map(|s| s.as_str()));
            foo.save_changes::<Podcast>(con)?;
        },
        Err(_) => {
            diesel::insert(pd)
                .into(schema::podcast::table)
                .execute(con)?;
        }
    }
    Ok(())
}

fn index_episode(con: &SqliteConnection, ep: &NewEpisode) -> Result<()> {
    match dbqueries::load_episode_from_uri(con, ep.uri.unwrap()) {
        Ok(mut foo) => if foo.title() != ep.title
            || foo.published_date() != ep.published_date.as_ref().map(|x| x.as_str())
        {
            foo.set_title(ep.title);
            foo.set_description(ep.description);
            foo.set_published_date(ep.published_date.as_ref().map(|x| x.as_str()));
            foo.set_guid(ep.guid);
            foo.set_length(ep.length);
            foo.set_epoch(ep.epoch);
            foo.save_changes::<Episode>(con)?;
        },
        Err(_) => {
            diesel::insert(ep)
                .into(schema::episode::table)
                .execute(con)?;
        }
    }
    Ok(())
}

pub fn insert_return_source(con: &SqliteConnection, url: &str) -> Result<Source> {
    let foo = NewSource::new_with_uri(url);
    index_source(con, &foo)?;

    Ok(dbqueries::load_source_from_uri(con, foo.uri)?)
}

fn insert_return_podcast(con: &SqliteConnection, pd: &NewPodcast) -> Result<Podcast> {
    index_podcast(con, pd)?;

    Ok(dbqueries::load_podcast_from_title(con, &pd.title)?)
}

fn insert_return_episode(con: &SqliteConnection, ep: &NewEpisode) -> Result<Episode> {
    index_episode(con, ep)?;

    Ok(dbqueries::load_episode_from_uri(con, ep.uri.unwrap())?)
}

pub fn full_index_loop(db: &Database) -> Result<()> {
    let mut f = fetch_feeds(db)?;

    index_feed(&db, &mut f);
    info!("Indexing done.");
    Ok(())
}

pub fn index_feed(db: &Database, f: &mut [Feed]) {
    f.par_iter_mut()
        .for_each(|&mut Feed(ref mut req, ref source)| {
            let e = complete_index_from_source(req, source, db);
            if e.is_err() {
                error!("Error While trying to update the database.");
                error!("Error msg: {}", e.unwrap_err());
            };
        });
}

pub fn complete_index_from_source(
    req: &mut reqwest::Response,
    source: &Source,
    db: &Database,
) -> Result<()> {
    use std::io::Read;
    use std::str::FromStr;

    let mut buf = String::new();
    req.read_to_string(&mut buf)?;
    let chan = rss::Channel::from_str(&buf)?;

    complete_index(db, &chan, source)?;

    Ok(())
}

fn complete_index(db: &Database, chan: &rss::Channel, parent: &Source) -> Result<()> {
    let pd = {
        let conn = db.lock().unwrap();
        index_channel(&conn, chan, parent)?
    };
    index_channel_items(db, chan.items(), &pd);
    Ok(())
}

fn index_channel(con: &SqliteConnection, chan: &rss::Channel, parent: &Source) -> Result<Podcast> {
    let pd = feedparser::parse_podcast(chan, parent.id());
    // Convert NewPodcast to Podcast
    let pd = insert_return_podcast(con, &pd)?;
    Ok(pd)
}

fn index_channel_items(db: &Database, it: &[rss::Item], pd: &Podcast) {
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

    if let Err(err) = e {
        error!("Episodes Transcaction Failed.");
        error!("Error msg: {}", err);
    };
}

// Maybe this can be refactored into an Iterator for lazy evaluation.
pub fn fetch_feeds(db: &Database) -> Result<Vec<Feed>> {
    let mut feeds = {
        let conn = db.lock().unwrap();
        dbqueries::get_sources(&conn)?
    };

    let results: Vec<Feed> = feeds
        .par_iter_mut()
        .filter_map(|x| {
            let l = refresh_source(db, x);
            if l.is_ok() {
                l.ok()
            } else {
                error!("Error While trying to fetch from source: {}.", x.uri());
                error!("Error msg: {}", l.unwrap_err());
                None
            }
        })
        .collect();

    Ok(results)
}

pub fn refresh_source(db: &Database, feed: &mut Source) -> Result<Feed> {
    use reqwest::header::{ETag, EntityTag, Headers, HttpDate, LastModified};

    let client = reqwest::Client::new();
    let mut headers = Headers::new();

    if let Some(foo) = feed.http_etag() {
        headers.set(ETag(EntityTag::new(true, foo.to_owned())));
    }

    if let Some(foo) = feed.last_modified() {
        if let Ok(x) = foo.parse::<HttpDate>() {
            headers.set(LastModified(x));
        }
    }

    // FIXME: I have fucked up somewhere here.
    // Getting back 200 codes even though I supposedly sent etags.
    // info!("Headers: {:?}", headers);
    let req = client.get(feed.uri()).headers(headers).send()?;

    info!("GET to {} , returned: {}", feed.uri(), req.status());

    // TODO match on more stuff
    // 301: Permanent redirect of the url
    // 302: Temporary redirect of the url
    // 304: Up to date Feed, checked with the Etag
    // 410: Feed deleted
    // match req.status() {
    //     reqwest::StatusCode::NotModified => (),
    //     _ => (),
    // };

    feed.update_etag(db, &req)?;
    Ok(Feed(req, feed.clone()))
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
            index_source(&tempdb, &NewSource::new_with_uri(feed)).unwrap()
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

        urls.iter().for_each(|&(path, url)| {
            let tempdb = m.lock().unwrap();
            // Create and insert a Source into db
            let s = insert_return_source(&tempdb, url).unwrap();
            drop(tempdb);

            // open the xml file
            let feed = fs::File::open(path).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(feed)).unwrap();

            // Index the channel
            complete_index(&m, &chan, &s).unwrap();
        });

        // Assert the index rows equal the controlled results
        let tempdb = m.lock().unwrap();
        assert_eq!(dbqueries::get_sources(&tempdb).unwrap().len(), 4);
        assert_eq!(dbqueries::get_podcasts(&tempdb).unwrap().len(), 4);
        assert_eq!(dbqueries::get_episodes(&tempdb).unwrap().len(), 274);
    }
}
