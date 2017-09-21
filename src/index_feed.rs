use diesel::prelude::*;
use diesel;
use rss;

use schema;
use dbqueries;
use feedparser;
use errors::*;
use models::{NewEpisode, NewSource, Source, Podcast};

pub fn foo() {
    let inpt = vec![
        "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
        "http://feeds.feedburner.com/linuxunplugged",
        "http://feeds.propublica.org/propublica/main",
    ];

    let db = ::establish_connection();
    for feed in inpt.iter() {
        match insert_source(&db, feed) {
            Ok(_) => {}
            Err(foo) => {
                debug!("Error: {}", foo);
                debug!("Skipping...");
                continue;
            }
        }
    }

    index_loop(db).unwrap();
}

fn insert_source(con: &SqliteConnection, url: &str) -> Result<()> {
    let foo = NewSource::new_with_uri(url);

    match dbqueries::load_source(con, foo.uri) {
        Ok(mut bar) => {
            // FIXME: NewSource has None values for etag, and last_mod atm
            // bar.set_http_etag(foo.http_etag.map(|x| x.to_string()));
            // bar.set_last_modified(foo.last_modified.map(|x| x.to_string()));
            // bar.save_changes::<Source>(con)?;
        }
        Err(_) => {
            diesel::insert(&foo).into(schema::source::table).execute(
                con,
            )?;
        }
    }

    Ok(())
}

fn index_podcast(con: &SqliteConnection, channel: &rss::Channel, parent: &Source) -> Result<()> {
    let pd = feedparser::parse_podcast(channel, parent.id())?;

    match dbqueries::load_podcast(con, &pd.title) {
        Ok(mut bar) => {
            bar.set_link(pd.link);
            bar.set_description(pd.description);
            bar.set_image_uri(pd.image_uri.map(|x| x.to_string()));
            bar.save_changes::<Podcast>(con)?;
        } 
        Err(_) => {
            diesel::insert(&pd).into(schema::podcast::table).execute(
                con,
            )?;
        }
    }

    Ok(())
}


pub fn index_loop(db: SqliteConnection) -> Result<()> {
    // let db = ::establish_connection();
    use feedparser;

    let f = dbqueries::get_sources(&db);

    for feed in f.unwrap().iter_mut() {
        // info!("{:?}", feed.id());

        // This method will defently get split and nuked
        // but for now its poc
        let chan = feed.get_podcast_chan(&db)?;
        let pd = feedparser::parse_podcast(&chan, feed.id())?;

        index_podcast(&db, &chan, &feed)?;

        // TODO: Separate the insert/update logic
        // diesel::insert_or_replace(&pd)
        //     .into(schema::podcast::table)
        //     .execute(&db)?;

        // Holy shit this works!
        let episodes: Vec<_> = chan.items()
            .iter()
            .map(|x| feedparser::parse_episode(x, feed.id()).unwrap())
            .collect();

        // lazy invoking the compiler to check for the Vec type :3
        // let first: &NewEpisode = episodes.first().unwrap();

        diesel::insert_or_replace(&episodes)
            .into(schema::episode::table)
            .execute(&db)?;

        info!("{:#?}", pd);
        // info!("{:#?}", episodes);
        // info!("{:?}", chan);

    }
    Ok(())
}