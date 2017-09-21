use diesel::prelude::*;
use diesel;
use schema;
use dbqueries;
use errors::*;
use models::{NewEpisode, NewSource, Source};

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

fn insert_source(connection: &SqliteConnection, url: &str) -> Result<()> {
    let foo = NewSource::new_with_uri(url);

    diesel::insert_or_replace(&foo)
        .into(schema::source::table)
        .execute(connection)?;

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

        // TODO: Separate the insert/update logic
        diesel::insert_or_replace(&pd)
            .into(schema::podcast::table)
            .execute(&db)?;

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