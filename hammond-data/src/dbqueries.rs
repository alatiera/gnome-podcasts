
use diesel::prelude::*;
use diesel;
use models::{Episode, Podcast, Source};
use Database;
use chrono::prelude::*;

/// Random db querries helper functions.
/// Probably needs cleanup.

pub fn get_sources(con: &SqliteConnection) -> QueryResult<Vec<Source>> {
    use schema::source::dsl::*;

    source.load::<Source>(con)
}

pub fn get_podcasts(con: &SqliteConnection) -> QueryResult<Vec<Podcast>> {
    use schema::podcast::dsl::*;

    podcast.load::<Podcast>(con)
}

pub fn get_episodes(con: &SqliteConnection) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    episode.order(epoch.desc()).load::<Episode>(con)
}

pub fn get_downloaded_episodes(con: &SqliteConnection) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    episode.filter(local_uri.is_not_null()).load::<Episode>(con)
}

pub fn get_played_episodes(con: &SqliteConnection) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    episode.filter(played.is_not_null()).load::<Episode>(con)
}

pub fn get_episode_from_id(con: &SqliteConnection, ep_id: i32) -> QueryResult<Episode> {
    use schema::episode::dsl::*;

    episode.filter(id.eq(ep_id)).get_result::<Episode>(con)
}

pub fn get_episode_local_uri_from_id(
    con: &SqliteConnection,
    ep_id: i32,
) -> QueryResult<Option<String>> {
    use schema::episode::dsl::*;

    episode
        .filter(id.eq(ep_id))
        .select(local_uri)
        .get_result::<Option<String>>(con)
}

pub fn get_episodes_with_limit(con: &SqliteConnection, limit: u32) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    episode
        .order(epoch.desc())
        .limit(i64::from(limit))
        .load::<Episode>(con)
}

pub fn get_podcast_from_id(con: &SqliteConnection, pid: i32) -> QueryResult<Podcast> {
    use schema::podcast::dsl::*;

    podcast.filter(id.eq(pid)).get_result::<Podcast>(con)
}

pub fn get_pd_episodes(con: &SqliteConnection, parent: &Podcast) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    Episode::belonging_to(parent)
        .order(epoch.desc())
        .load::<Episode>(con)
}

pub fn get_pd_unplayed_episodes(
    con: &SqliteConnection,
    parent: &Podcast,
) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    Episode::belonging_to(parent)
        .filter(played.is_null())
        .order(epoch.desc())
        .load::<Episode>(con)
}

pub fn get_pd_episodes_limit(
    con: &SqliteConnection,
    parent: &Podcast,
    limit: u32,
) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    Episode::belonging_to(parent)
        .order(epoch.desc())
        .limit(i64::from(limit))
        .load::<Episode>(con)
}

pub fn get_source_from_uri(con: &SqliteConnection, uri_: &str) -> QueryResult<Source> {
    use schema::source::dsl::*;

    source.filter(uri.eq(uri_)).get_result::<Source>(con)
}

pub fn get_podcast_from_title(con: &SqliteConnection, title_: &str) -> QueryResult<Podcast> {
    use schema::podcast::dsl::*;

    podcast.filter(title.eq(title_)).get_result::<Podcast>(con)
}

pub fn get_episode_from_uri(con: &SqliteConnection, uri_: &str) -> QueryResult<Episode> {
    use schema::episode::dsl::*;

    episode.filter(uri.eq(uri_)).get_result::<Episode>(con)
}

pub fn remove_feed(db: &Database, pd: &Podcast) -> QueryResult<usize> {
    let tempdb = db.lock().unwrap();

    tempdb.transaction(|| -> QueryResult<usize> {
        delete_source(&tempdb, pd.source_id())?;
        delete_podcast(&tempdb, pd.id())?;
        delete_podcast_episodes(&tempdb, pd.id())
    })
}

pub fn delete_source(connection: &SqliteConnection, source_id: i32) -> QueryResult<usize> {
    use schema::source::dsl::*;

    diesel::delete(source.filter(id.eq(source_id))).execute(connection)
}

pub fn delete_podcast(connection: &SqliteConnection, podcast_id: i32) -> QueryResult<usize> {
    use schema::podcast::dsl::*;

    diesel::delete(podcast.filter(id.eq(podcast_id))).execute(connection)
}

pub fn delete_podcast_episodes(
    connection: &SqliteConnection,
    parent_id: i32,
) -> QueryResult<usize> {
    use schema::episode::dsl::*;

    diesel::delete(episode.filter(podcast_id.eq(parent_id))).execute(connection)
}

pub fn update_none_to_played_now(
    connection: &SqliteConnection,
    parent: &Podcast,
) -> QueryResult<usize> {
    use schema::episode::dsl::*;

    let epoch_now = Utc::now().timestamp() as i32;
    connection.transaction(|| -> QueryResult<usize> {
        diesel::update(Episode::belonging_to(parent).filter(played.is_null()))
            .set(played.eq(Some(epoch_now)))
            .execute(connection)
    })
}
