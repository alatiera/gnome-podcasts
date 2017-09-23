use diesel::prelude::*;
use diesel;
use rss;
use reqwest;

use schema;
use dbqueries;
use feedparser;
use errors::*;
use models::{Episode, NewSource, Podcast, Source};

pub fn foo() {
    let inpt = vec![
        "https://request-for-explanation.github.io/podcast/rss.xml",
        "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
        "http://feeds.propublica.org/propublica/podcast",
        "http://feeds.feedburner.com/linuxunplugged",
    ];

    let db = ::establish_connection();
    for feed in inpt.iter() {
        match insert_source(&db, feed) {
            Ok(_) => {}
            Err(foo) => {
                error!("Error: {}", foo);
                continue;
            }
        }
    }

    index_loop(&db).unwrap();
}

fn insert_source(con: &SqliteConnection, url: &str) -> Result<Source> {
    let foo = NewSource::new_with_uri(url);

    match dbqueries::load_source(con, foo.uri) {
        Ok(_) => (),
        Err(_) => {
            diesel::insert(&foo)
                .into(schema::source::table)
                .execute(con)?;
        }
    }

    Ok(dbqueries::load_source(con, foo.uri)?)
}

fn index_podcast(
    con: &SqliteConnection,
    channel: &rss::Channel,
    parent: &Source,
) -> Result<Podcast> {
    let pd = feedparser::parse_podcast(channel, parent.id())?;

    match dbqueries::load_podcast(con, &pd.title) {
        Ok(mut foo) => if foo.link() != pd.link || foo.description() != pd.description {
            foo.set_link(&pd.link);
            foo.set_description(&pd.description);
            foo.set_image_uri(pd.image_uri.as_ref().map(|s| s.as_str()));
            foo.save_changes::<Podcast>(con)?;
        },
        Err(_) => {
            diesel::insert(&pd)
                .into(schema::podcast::table)
                .execute(con)?;
        }
    }

    Ok(dbqueries::load_podcast(con, &pd.title)?)
}

fn index_episode(con: &SqliteConnection, item: &rss::Item, parent: &Podcast) -> Result<Episode> {
    let ep = feedparser::parse_episode(item, parent.id())?;

    match dbqueries::load_episode(con, &ep.uri.unwrap()) {
        Ok(mut foo) => if foo.title() != ep.title || foo.published_date() != ep.published_date {
            foo.set_title(ep.title);
            foo.set_description(ep.description);
            foo.set_published_date(ep.published_date);
            foo.set_guid(ep.guid);
            foo.set_length(ep.length);
            foo.set_epoch(ep.epoch);
            foo.save_changes::<Episode>(con)?;
        },
        Err(_) => {
            diesel::insert(&ep)
                .into(schema::episode::table)
                .execute(con)?;
        }
    }

    Ok(dbqueries::load_episode(con, &ep.uri.unwrap())?)
}

pub fn index_loop(db: &SqliteConnection) -> Result<()> {
    use std::io::Read;
    use std::str::FromStr;

    let mut f = fetch_feeds(db)?;

    for &mut (ref mut req, ref source) in f.iter_mut() {
        let mut buf = String::new();
        req.read_to_string(&mut buf)?;
        let chan = rss::Channel::from_str(&buf)?;

        let pd = index_podcast(db, &chan, source)?;

        let _: Vec<_> = chan.items()
            .iter()
            .map(|x| index_episode(db, &x, &pd))
            .collect();

        info!("{:#?}", pd);
        // info!("{:#?}", episodes);
        // info!("{:?}", chan);
    }
    Ok(())
}

// TODO: make it into an iterator that yields reqwest::response
// TODO: After fixing etag/lmod, add sent_etag:bool arg and logic to bypass it.
pub fn fetch_feeds(connection: &SqliteConnection) -> Result<Vec<(reqwest::Response, Source)>> {
    use reqwest::header::{ETag, EntityTag, Headers, HttpDate, LastModified};

    let mut results = Vec::new();

    let mut feeds = dbqueries::get_sources(connection)?;

    for feed in feeds.iter_mut() {
        let client = reqwest::Client::new()?;
        let mut headers = Headers::new();

        if let Some(foo) = feed.http_etag() {
            headers.set(ETag(EntityTag::new(true, foo.to_owned())));
        }

        if let Some(foo) = feed.last_modified() {
            headers.set(LastModified(foo.parse::<HttpDate>()?));
        }

        info!("{:?}", headers);
        // FIXME: I have fucked up something here.
        // Getting back 200 codes even though I supposedly sent etags.
        let req = client.get(feed.uri())?.headers(headers).send()?;
        info!("{}", req.status());

        // TODO match on more stuff
        match req.status() {
            reqwest::StatusCode::NotModified => {
                continue;
            }
            _ => (),
        };

        feed.update_etag(connection, &req)?;
        results.push((req, feed.clone()));
    }

    Ok(results)
}
