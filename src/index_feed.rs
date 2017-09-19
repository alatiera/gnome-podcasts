use diesel::prelude::*;
use diesel;
use schema;
use dbqueries;
use errors::*;
use models::{NewPodcast, NewSource};

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

    let f = dbqueries::get_sources(&db);
    info!("{:?}", f);
}

fn insert_source(connection: &SqliteConnection, url: &str) -> Result<()> {
    let foo = NewSource::new_with_uri(url);

    diesel::insert(&foo).into(schema::source::table).execute(
        connection,
    )?;

    Ok(())
}
