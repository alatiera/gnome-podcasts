use diesel::prelude::*;
use schema::source::dsl::*;
use models::{Podcast, Episode, Source};

pub fn get_podcasts(con: &SqliteConnection, parent: &Source) -> QueryResult<Vec<Podcast>> {
    let pds = Podcast::belonging_to(parent).load::<Podcast>(con);
    // debug!("Returned Podcasts:\n{:?}", pds);
    pds
}

pub fn get_pd_episodes(con: &SqliteConnection, parent: &Podcast) -> QueryResult<Vec<Episode>> {
    let eps = Episode::belonging_to(parent).load::<Episode>(con);
    eps
}

 pub fn get_sources(con: &SqliteConnection) -> QueryResult<Vec<Source>>{
     let s = source.load::<Source>(con);
     s
 }