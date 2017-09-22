use diesel::prelude::*;
use diesel;
use rss;

use schema;
use dbqueries;
use feedparser;
use errors::*;
use models::{NewSource, Source, Podcast, Episode};

pub fn foo() {
    let inpt = vec![
        "https://request-for-explanation.github.io/podcast/rss.xml",
        "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
        "http://feeds.propublica.org/propublica/main",
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

    index_loop(db).unwrap();
}

fn insert_source(con: &SqliteConnection, url: &str) -> Result<Source> {
    let foo = NewSource::new_with_uri(url);

    match dbqueries::load_source(con, foo.uri) {
        Ok(mut bar) => {
            // TODO: Cmp first before replacing
            // FIXME: NewSource has None values for etag, and last_mod atm
            // bar.set_http_etag(foo.http_etag);
            // bar.set_last_modified(foo.last_modified);
            // bar.save_changes::<Source>(con)?;
        }
        Err(_) => {
            diesel::insert(&foo).into(schema::source::table).execute(
                con,
            )?;
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
        Ok(mut foo) => {
            // TODO: Cmp first before replacing
            foo.set_link(&pd.link);
            foo.set_description(&pd.description);
            foo.set_image_uri(pd.image_uri.as_ref().map(|s| s.as_str()));
            foo.save_changes::<Podcast>(con)?;
        }
        Err(_) => {
            diesel::insert(&pd).into(schema::podcast::table).execute(
                con,
            )?;
        }
    }

    Ok(dbqueries::load_podcast(con, &pd.title)?)
}

fn index_episode(con: &SqliteConnection, item: &rss::Item, parent: &Podcast) -> Result<Episode> {
    let ep = feedparser::parse_episode(item, parent.id())?;

    match dbqueries::load_episode(con, &ep.uri.unwrap()) {
        Ok(mut foo) => {
            // TODO: Cmp first before replacing
            foo.set_title(ep.title);
            foo.set_description(ep.description);
            foo.set_published_date(ep.published_date);
            foo.set_guid(ep.guid);
            foo.set_length(ep.length);
            foo.set_epoch(ep.length);
            foo.save_changes::<Episode>(con)?;
        } 
        Err(_) => {
            diesel::insert(&ep).into(schema::episode::table).execute(
                con,
            )?;
        }
    }

    Ok(dbqueries::load_episode(con, &ep.uri.unwrap())?)
}

pub fn index_loop(db: SqliteConnection) -> Result<()> {
    // let db = ::establish_connection();

    let f = dbqueries::get_sources(&db);

    for feed in f.unwrap().iter_mut() {
        // info!("{:?}", feed.id());

        // This method will defently get split and nuked
        // but for now its poc
        let chan = feed.get_podcast_chan(&db)?;

        let pd = index_podcast(&db, &chan, &feed)?;

        let _: Vec<_> = chan.items()
            .iter()
            .map(|x| index_episode(&db, &x, &pd))
            .collect();

        info!("{:#?}", pd);
        // info!("{:#?}", episodes);
        // info!("{:?}", chan);

    }
    Ok(())
}