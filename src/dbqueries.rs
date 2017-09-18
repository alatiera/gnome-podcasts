use diesel::prelude::*;
use schema::podcast::dsl::*;
// use schema::episode::dsl::*;
use models::{Podcast, Episode};

pub fn get_podcasts(con: &SqliteConnection) -> QueryResult<Vec<Podcast>> {
    let pds = podcast.load::<Podcast>(con);
    // debug!("Returned Podcasts:\n{:?}", pds);
    pds
}

pub fn get_pd_episodes(con: &SqliteConnection, parent: &Podcast) -> QueryResult<Vec<Episode>> {
    let eps = Episode::belonging_to(parent).load::<Episode>(con);
    eps
}
 