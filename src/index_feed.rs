use diesel::prelude::*;
use diesel;
use rss;
use reqwest;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

use schema;
use dbqueries;
use feedparser;
use errors::*;
use models::*;

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
    match dbqueries::load_episode(con, &ep.uri.unwrap()) {
        Ok(mut foo) => if foo.title() != ep.title
            || foo.published_date() != ep.published_date.as_ref().map(|x| x.as_str())
        {
            foo.set_title(ep.title);
            foo.set_description(ep.description);
            foo.set_published_date(ep.published_date.clone());
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
fn insert_return_source(con: &SqliteConnection, url: &str) -> Result<Source> {
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

    Ok(dbqueries::load_episode(con, &ep.uri.unwrap())?)
}

pub fn index_loop(db: SqliteConnection) -> Result<()> {
    let m = Arc::new(Mutex::new(db));

    let mut f = fetch_feeds(m.clone())?;

    f.par_iter_mut().for_each(|&mut (ref mut req, ref source)| {
        complete_index_from_source(req, source, m.clone()).unwrap()
    });

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
    let pd = feedparser::parse_podcast(&chan, source.id())?;

    let tempdb = mutex.lock().unwrap();
    let pd = insert_return_podcast(&tempdb, &pd)?;
    drop(tempdb);

    let foo: Vec<_> = chan.items()
        .par_iter()
        .map(|x| feedparser::parse_episode(&x, pd.id()).unwrap())
        .collect();

    foo.par_iter().for_each(|x| {
        let dbmutex = mutex.clone();
        let db = dbmutex.lock().unwrap();
        index_episode(&db, &x).unwrap();
    });

    Ok(())
}

// TODO: maybe refactor into an Iterator for lazy evaluation.
// TODO: After fixing etag/lmod, add sent_etag:bool arg and logic to bypass it.
pub fn fetch_feeds(
    connection: Arc<Mutex<SqliteConnection>>,
) -> Result<Vec<(reqwest::Response, Source)>> {
    let tempdb = connection.lock().unwrap();
    let mut feeds = dbqueries::get_sources(&tempdb)?;
    drop(tempdb);

    let results: Vec<_> = feeds
        .par_iter_mut()
        .map(|x| {
            let dbmutex = connection.clone();
            let db = dbmutex.lock().unwrap();
            refresh_source(&db, x).unwrap()
        })
        .collect();

    Ok(results)
}

fn refresh_source(
    connection: &SqliteConnection,
    feed: &mut Source,
) -> Result<(reqwest::Response, Source)> {
    use reqwest::header::{ETag, EntityTag, Headers, HttpDate, LastModified};

    let client = reqwest::Client::new()?;
    let mut headers = Headers::new();

    if let Some(foo) = feed.http_etag() {
        headers.set(ETag(EntityTag::new(true, foo.to_owned())));
    }

    if let Some(foo) = feed.last_modified() {
        headers.set(LastModified(foo.parse::<HttpDate>()?));
    }

    info!("Headers: {:?}", headers);
    // FIXME: I have fucked up somewhere here.
    // Getting back 200 codes even though I supposedly sent etags.
    let req = client.get(feed.uri())?.headers(headers).send()?;
    info!("{}", req.status());

    // TODO match on more stuff
    // 301: Permanent redirect of the url
    // 302: Temporary redirect of the url
    // 304: Up to date Feed, checked with the Etag
    // 410: Feed deleted
    match req.status() {
        reqwest::StatusCode::NotModified => (),
        _ => (),
    };

    feed.update_etag(connection, &req)?;
    Ok((req, feed.clone()))
}

#[cfg(test)]
mod tests {
    extern crate tempdir;
    use diesel::prelude::*;

    use std::io::stdout;
    use std::path::PathBuf;

    use super::*;

    embed_migrations!("migrations/");
    // struct TempDB {
    //     tmp_dir: tempdir::TempDir,
    //     db_path: PathBuf,
    //     db: SqliteConnection,
    // }
    struct TempDB(tempdir::TempDir, PathBuf, SqliteConnection);

    /// Create and return a Temporary DB.
    /// Will be destroed once the returned variable(s) is dropped.
    fn get_temp_db() -> TempDB {
        let tmp_dir = tempdir::TempDir::new("hammond_unit_test").unwrap();
        let db_path = tmp_dir.path().join("foo_tests.db");

        let db = SqliteConnection::establish(db_path.to_str().unwrap()).unwrap();
        embedded_migrations::run_with_output(&db, &mut stdout()).unwrap();

        // TempDB {
        //     tmp_dir,
        //     db_path,
        //     db,
        // }
        TempDB(tmp_dir, db_path, db)
    }

    #[test]
    /// Insert feeds and update/index them.
    fn foo() {
        let TempDB(_tmp_dir, db_path, db) = get_temp_db();

        let inpt = vec![
            "https://request-for-explanation.github.io/podcast/rss.xml",
            "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
            "http://feeds.propublica.org/propublica/podcast",
            "http://feeds.feedburner.com/linuxunplugged",
        ];

        inpt.iter().for_each(|feed| {
            index_source(&db, &NewSource::new_with_uri(feed)).unwrap()
        });
        index_loop(db).unwrap();

        // index_loop takes oweneship of the dbconnection in order to create mutexes.
        let db = SqliteConnection::establish(db_path.to_str().unwrap()).unwrap();

        // Run again to cover Unique constrains erros.
        index_loop(db).unwrap();
    }

    // #[test]
    // fn baz(){
    //     let TempDB(tmp_dir, db_path, db) = get_temp_db();

    // }
}
