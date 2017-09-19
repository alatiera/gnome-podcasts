use diesel::prelude::*;
use diesel;
use schema;
use dbqueries;
use errors::*;
use models::{NewPodcast, NewSource, Source};

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

    index_loop(db);
}

fn insert_source(connection: &SqliteConnection, url: &str) -> Result<()> {
    let foo = NewSource::new_with_uri(url);

    diesel::insert(&foo).into(schema::source::table).execute(
        connection,
    )?;

    Ok(())
}


pub fn index_loop(db: SqliteConnection) -> Result<()> {
    // let db = ::establish_connection();
    use parse_feeds;

    let f = dbqueries::get_sources(&db);

    for feed in f.unwrap().iter_mut() {
        info!("{:?}", feed.id());
        // This method will defently get split and nuked
        // but for now its poc
        let chan = feed.get_podcast_chan(&db)?;
        let pd = parse_feeds::parse_podcast(&chan, feed.id())?;
        info!("{:#?}", pd);
        // info!("{:?}", chan);

    }
    Ok(())
}