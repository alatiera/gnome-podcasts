#![allow(dead_code)]
#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]

use diesel::prelude::*;
use diesel;
use rss;
use reqwest;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

use schema;
use dbqueries;
use models::*;
use errors::*;
use feedparser;

fn index_source(con: &SqliteConnection, foo: &NewSource) -> Result<()> {
    match dbqueries::load_source(con, foo.uri) {
        Ok(_) => Ok(()),
        Err(_) => {
            diesel::insert(foo).into(schema::source::table).execute(con)?;
            Ok(())
        }
    }
}

fn index_podcast(con: &SqliteConnection, pd: &NewPodcast) -> Result<()> {
    match dbqueries::load_podcast(con, &pd.title) {
        Ok(mut foo) => if foo.link() != pd.link || foo.description() != pd.description {
            foo.set_link(&pd.link);
            foo.set_description(&pd.description);
            foo.set_image_uri(pd.image_uri.as_ref().map(|s| s.as_str()));
            foo.save_changes::<Podcast>(con)?;
        },
        Err(_) => {
            diesel::insert(pd).into(schema::podcast::table).execute(con)?;
        }
    }
    Ok(())
}

fn index_episode(con: &SqliteConnection, ep: &NewEpisode) -> Result<()> {
    match dbqueries::load_episode(con, ep.uri.unwrap()) {
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
            diesel::insert(ep).into(schema::episode::table).execute(con)?;
        }
    }
    Ok(())
}

pub fn insert_return_source(con: &SqliteConnection, url: &str) -> Result<Source> {
    let foo = NewSource::new_with_uri(url);
    index_source(con, &foo)?;

    Ok(dbqueries::load_source(con, foo.uri)?)
}

fn insert_return_podcast(con: &SqliteConnection, pd: &NewPodcast) -> Result<Podcast> {
    index_podcast(con, pd)?;

    Ok(dbqueries::load_podcast(con, &pd.title)?)
}

fn insert_return_episode(con: &SqliteConnection, ep: &NewEpisode) -> Result<Episode> {
    index_episode(con, ep)?;

    Ok(dbqueries::load_episode(con, ep.uri.unwrap())?)
}

pub fn index_loop(db: SqliteConnection, force: bool) -> Result<()> {
    let m = Arc::new(Mutex::new(db));

    let mut f = fetch_feeds(m.clone(), force)?;

    // f.par_iter_mut().for_each(|&mut (ref mut req, ref source)| {
    // TODO: Once for_each is stable, uncomment above line and delete collect.
    let _: Vec<_> = f.par_iter_mut()
        .map(|&mut (ref mut req, ref source)| {
            complete_index_from_source(req, source, m.clone()).unwrap();
        })
        .collect();

    Ok(())
}

fn complete_index_from_source(
    req: &mut reqwest::Response,
    source: &Source,
    mutex: Arc<Mutex<SqliteConnection>>,
) -> Result<()> {
    use std::io::Read;
    use std::str::FromStr;

    let mut buf = String::new();
    req.read_to_string(&mut buf)?;
    let chan = rss::Channel::from_str(&buf)?;

    complete_index(mutex, &chan, source)?;

    Ok(())
}

fn complete_index(
    mutex: Arc<Mutex<SqliteConnection>>,
    chan: &rss::Channel,
    parent: &Source,
) -> Result<()> {
    let tempdb = mutex.lock().unwrap();
    let pd = index_channel(&tempdb, chan, parent)?;
    drop(tempdb);

    index_channel_items(mutex.clone(), chan.items(), &pd)?;

    Ok(())
}

fn index_channel(db: &SqliteConnection, chan: &rss::Channel, parent: &Source) -> Result<Podcast> {
    let pd = feedparser::parse_podcast(chan, parent.id())?;
    // Convert NewPodcast to Podcast
    let pd = insert_return_podcast(db, &pd)?;
    Ok(pd)
}

// TODO: Propagate the erros from the maps up the chain.
fn index_channel_items(
    mutex: Arc<Mutex<SqliteConnection>>,
    i: &[rss::Item],
    pd: &Podcast,
) -> Result<()> {
    let foo: Vec<_> = i.par_iter()
        .map(|x| feedparser::parse_episode(x, pd.id()).unwrap())
        .collect();

    foo.par_iter().for_each(|x| {
        let dbmutex = mutex.clone();
        let db = dbmutex.lock().unwrap();
        index_episode(&db, x).unwrap();
    });
    Ok(())
}

// Maybe this can be refactored into an Iterator for lazy evaluation.
pub fn fetch_feeds(
    connection: Arc<Mutex<SqliteConnection>>,
    force: bool,
) -> Result<Vec<(reqwest::Response, Source)>> {
    let tempdb = connection.lock().unwrap();
    let mut feeds = dbqueries::get_sources(&tempdb)?;
    drop(tempdb);

    let results: Vec<_> = feeds
        .par_iter_mut()
        .map(|x| {
            let dbmutex = connection.clone();
            let db = dbmutex.lock().unwrap();
            refresh_source(&db, x, force).unwrap()
        })
        .collect();

    Ok(results)
}

fn refresh_source(
    connection: &SqliteConnection,
    feed: &mut Source,
    force: bool,
) -> Result<(reqwest::Response, Source)> {
    use reqwest::header::{ETag, EntityTag, Headers, HttpDate, LastModified};

    let client = reqwest::Client::new()?;
    let req;
    match force {
        true => req = client.get(feed.uri())?.send()?,
        false => {
            let mut headers = Headers::new();

            if let Some(foo) = feed.http_etag() {
                headers.set(ETag(EntityTag::new(true, foo.to_owned())));
            }

            if let Some(foo) = feed.last_modified() {
                headers.set(LastModified(foo.parse::<HttpDate>()?));
            }

            // FIXME: I have fucked up somewhere here.
            // Getting back 200 codes even though I supposedly sent etags.
            info!("Headers: {:?}", headers);
            req = client.get(feed.uri())?.headers(headers).send()?;
        }
    };

    info!("{}", req.status());

    // TODO match on more stuff
    // 301: Permanent redirect of the url
    // 302: Temporary redirect of the url
    // 304: Up to date Feed, checked with the Etag
    // 410: Feed deleted
    // match req.status() {
    //     reqwest::StatusCode::NotModified => (),
    //     _ => (),
    // };

    feed.update_etag(connection, &req)?;
    Ok((req, feed.clone()))
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
        let TempDB(_tmp_dir, db_path, db) = get_temp_db();

        let inpt = vec![
            "https://request-for-explanation.github.io/podcast/rss.xml",
            "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
            "http://feeds.propublica.org/propublica/podcast",
            "http://feeds.feedburner.com/linuxunplugged",
        ];

        inpt.iter()
            .map(|feed| {
                index_source(&db, &NewSource::new_with_uri(feed)).unwrap()
            })
            .fold((), |(), _| ());

        index_loop(db, true).unwrap();

        // index_loop takes oweneship of the dbconnection in order to create mutexes.
        let db = SqliteConnection::establish(db_path.to_str().unwrap()).unwrap();

        // Run again to cover Unique constrains erros.
        index_loop(db, true).unwrap();
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

        urls.iter()
            .map(|&(path, url)| {
                let tempdb = m.lock().unwrap();
                // Create and insert a Source into db
                let s = insert_return_source(&tempdb, url).unwrap();
                drop(tempdb);

                // open the xml file
                let feed = fs::File::open(path).unwrap();
                // parse it into a channel
                let chan = rss::Channel::read_from(BufReader::new(feed)).unwrap();

                // Index the channel
                complete_index(m.clone(), &chan, &s).unwrap();
            })
            .fold((), |(), _| ());

        // Assert the index rows equal the controlled results
        let tempdb = m.lock().unwrap();
        assert_eq!(dbqueries::get_sources(&tempdb).unwrap().len(), 4);
        assert_eq!(dbqueries::get_podcasts(&tempdb).unwrap().len(), 4);
        assert_eq!(dbqueries::get_episodes(&tempdb).unwrap().len(), 274);
    }
}
