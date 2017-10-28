#![cfg_attr(feature = "cargo-clippy", allow(let_and_return))]

use diesel::prelude::*;
use diesel;
use models::{Episode, Podcast, Source};
use index_feed::Database;
use errors::*;

// TODO: Needs cleanup.

pub fn get_sources(con: &SqliteConnection) -> QueryResult<Vec<Source>> {
    use schema::source::dsl::*;

    let s = source.load::<Source>(con);
    s
}

pub fn get_podcasts(con: &SqliteConnection) -> QueryResult<Vec<Podcast>> {
    use schema::podcast::dsl::*;

    let pds = podcast.load::<Podcast>(con);
    pds
}

// Maybe later.
// pub fn get_podcasts_ids(con: &SqliteConnection) -> QueryResult<Vec<i32>> {
//     use schema::podcast::dsl::*;

//     let pds = podcast.select(id).load::<i32>(con);
//     pds
// }

pub fn get_episodes(con: &SqliteConnection) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let eps = episode.order(epoch.desc()).load::<Episode>(con);
    eps
}

pub fn get_downloaded_episodes(con: &SqliteConnection) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let eps = episode.filter(local_uri.is_not_null()).load::<Episode>(con);
    eps
}

pub fn get_watched_episodes(con: &SqliteConnection) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let eps = episode.filter(watched.is_not_null()).load::<Episode>(con);
    eps
}

pub fn get_episode(con: &SqliteConnection, ep_id: i32) -> QueryResult<Episode> {
    use schema::episode::dsl::*;

    let ep = episode.filter(id.eq(ep_id)).get_result::<Episode>(con);
    ep
}

pub fn get_episode_from_local_uri(
    con: &SqliteConnection,
    ep_id: i32,
) -> QueryResult<Option<String>> {
    use schema::episode::dsl::*;

    let ep = episode
        .filter(id.eq(ep_id))
        .select(local_uri)
        .get_result::<Option<String>>(con);
    ep
}

pub fn get_episodes_with_limit(con: &SqliteConnection, limit: u32) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let eps = episode
        .order(epoch.desc())
        .limit(i64::from(limit))
        .load::<Episode>(con);
    eps
}

pub fn get_podcast(con: &SqliteConnection, parent: &Source) -> QueryResult<Vec<Podcast>> {
    let pd = Podcast::belonging_to(parent).load::<Podcast>(con);
    // debug!("Returned Podcasts:\n{:?}", pds);
    pd
}

pub fn get_pd_episodes(con: &SqliteConnection, parent: &Podcast) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let eps = Episode::belonging_to(parent)
        .order(epoch.desc())
        .load::<Episode>(con);
    eps
}

pub fn get_pd_episodes_limit(
    con: &SqliteConnection,
    parent: &Podcast,
    limit: u32,
) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let eps = Episode::belonging_to(parent)
        .order(epoch.desc())
        .limit(i64::from(limit))
        .load::<Episode>(con);
    eps
}

pub fn load_source_from_uri(con: &SqliteConnection, uri_: &str) -> QueryResult<Source> {
    use schema::source::dsl::*;

    let s = source.filter(uri.eq(uri_)).get_result::<Source>(con);
    s
}

pub fn load_podcast_from_title(con: &SqliteConnection, title_: &str) -> QueryResult<Podcast> {
    use schema::podcast::dsl::*;

    let pd = podcast.filter(title.eq(title_)).get_result::<Podcast>(con);
    pd
}

pub fn load_episode_from_uri(con: &SqliteConnection, uri_: &str) -> QueryResult<Episode> {
    use schema::episode::dsl::*;

    let ep = episode.filter(uri.eq(uri_)).get_result::<Episode>(con);
    ep
}

pub fn remove_feed(db: &Database, pd: &Podcast) -> Result<()> {
    let s_id = pd.source_id();
    let pd_id = pd.id();
    let tempdb = db.lock().unwrap();

    tempdb.transaction(|| -> Result<()> {
        delete_source(&tempdb, s_id)?;
        delete_podcast(&tempdb, pd_id)?;
        delete_podcast_episodes(&tempdb, pd_id)?;
        Ok(())
    })?;
    Ok(())
}

pub fn delete_source(connection: &SqliteConnection, source_id: i32) -> Result<()> {
    use schema::source::dsl::*;

    diesel::delete(source.filter(id.eq(source_id))).execute(connection)?;
    Ok(())
}

pub fn delete_podcast(connection: &SqliteConnection, podcast_id: i32) -> Result<()> {
    use schema::podcast::dsl::*;

    diesel::delete(podcast.filter(id.eq(podcast_id))).execute(connection)?;
    Ok(())
}

pub fn delete_podcast_episodes(connection: &SqliteConnection, parent_id: i32) -> Result<()> {
    use schema::episode::dsl::*;

    diesel::delete(episode.filter(podcast_id.eq(parent_id))).execute(connection)?;
    Ok(())
}
