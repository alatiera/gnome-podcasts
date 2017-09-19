use diesel::prelude::*;
use diesel;
use rss::Channel;
use schema;
use dbqueries;
use errors::*;
use models::NewPodcast;

pub fn foo() {
    let inpt = vec![
        "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
        "http://feeds.feedburner.com/linuxunplugged",
        "http://feeds.propublica.org/propublica/main",
    ];

    let db = ::establish_connection();
    for feed in inpt.iter() {
        match insert_feed(&db, feed) {
            Ok(_) => {}
            Err(foo) => {
                debug!("Error: {}", foo);
                debug!("Skipping...");
                continue;
            }
        }
    }
    update_podcasts(&db);
}

fn insert_feed(connection: &SqliteConnection, url: &str) -> Result<()> {
    let foo = NewPodcast::from_url(url)?;

    diesel::insert(&foo).into(schema::podcast::table).execute(
        connection,
    )?;

    Ok(())
}


fn update_podcasts(connection: &SqliteConnection) {
    let pds = dbqueries::get_podcasts(connection).unwrap();
    info!("{:?}", pds);

    // for pd in pds {
    //     println!("{:?}" pd.uri);
    // }

}