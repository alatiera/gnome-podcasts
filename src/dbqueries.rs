use diesel::prelude::*;
use models::{Episode, Podcast, Source};

pub fn get_sources(con: &SqliteConnection) -> QueryResult<Vec<Source>> {
    use schema::source::dsl::*;

    let s = source.load::<Source>(con);
    s
}

pub fn get_podcasts(con: &SqliteConnection, parent: &Source) -> QueryResult<Vec<Podcast>> {
    let pds = Podcast::belonging_to(parent).load::<Podcast>(con);
    // debug!("Returned Podcasts:\n{:?}", pds);
    pds
}

pub fn get_pd_episodes(con: &SqliteConnection, parent: &Podcast) -> QueryResult<Vec<Episode>> {
    let eps = Episode::belonging_to(parent).load::<Episode>(con);
    eps
}

pub fn load_source(con: &SqliteConnection, uri_: &str) -> QueryResult<Source> {
    use schema::source::dsl::*;

    let s = source.filter(uri.eq(uri_)).get_result::<Source>(con);
    s
}

pub fn load_podcast(con: &SqliteConnection, title_: &str) -> QueryResult<Podcast> {
    use schema::podcast::dsl::*;

    let pd = podcast.filter(title.eq(title_)).get_result::<Podcast>(con);
    pd
}

pub fn load_episode(con: &SqliteConnection, uri_: &str) -> QueryResult<Episode> {
    use schema::episode::dsl::*;

    let ep = episode.filter(uri.eq(uri_)).get_result::<Episode>(con);
    ep
}
