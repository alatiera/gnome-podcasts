
use diesel::prelude::*;
use diesel;
use models::{Episode, Podcast, Source};
use chrono::prelude::*;

/// Random db querries helper functions.
/// Probably needs cleanup.

use POOL;

pub fn get_sources() -> QueryResult<Vec<Source>> {
    use schema::source::dsl::*;

    let con = POOL.clone().get().unwrap();
    source.load::<Source>(&*con)
}

pub fn get_podcasts() -> QueryResult<Vec<Podcast>> {
    use schema::podcast::dsl::*;

    let con = POOL.clone().get().unwrap();
    podcast.load::<Podcast>(&*con)
}

pub fn get_episodes() -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();
    episode.order(epoch.desc()).load::<Episode>(&*con)
}

pub fn get_downloaded_episodes() -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();
    episode
        .filter(local_uri.is_not_null())
        .load::<Episode>(&*con)
}

pub fn get_played_episodes() -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();
    episode.filter(played.is_not_null()).load::<Episode>(&*con)
}

pub fn get_episode_from_id(ep_id: i32) -> QueryResult<Episode> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();
    episode.filter(id.eq(ep_id)).get_result::<Episode>(&*con)
}

pub fn get_episode_local_uri_from_id(ep_id: i32) -> QueryResult<Option<String>> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();

    episode
        .filter(id.eq(ep_id))
        .select(local_uri)
        .get_result::<Option<String>>(&*con)
}

pub fn get_episodes_with_limit(limit: u32) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();

    episode
        .order(epoch.desc())
        .limit(i64::from(limit))
        .load::<Episode>(&*con)
}

pub fn get_podcast_from_id(pid: i32) -> QueryResult<Podcast> {
    use schema::podcast::dsl::*;

    let con = POOL.clone().get().unwrap();
    podcast.filter(id.eq(pid)).get_result::<Podcast>(&*con)
}

pub fn get_pd_episodes(parent: &Podcast) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();

    Episode::belonging_to(parent)
        .order(epoch.desc())
        .load::<Episode>(&*con)
}

pub fn get_pd_unplayed_episodes(parent: &Podcast) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();

    Episode::belonging_to(parent)
        .filter(played.is_null())
        .order(epoch.desc())
        .load::<Episode>(&*con)
}

pub fn get_pd_episodes_limit(parent: &Podcast, limit: u32) -> QueryResult<Vec<Episode>> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();

    Episode::belonging_to(parent)
        .order(epoch.desc())
        .limit(i64::from(limit))
        .load::<Episode>(&*con)
}

pub fn get_source_from_uri(uri_: &str) -> QueryResult<Source> {
    use schema::source::dsl::*;

    let con = POOL.clone().get().unwrap();
    source.filter(uri.eq(uri_)).get_result::<Source>(&*con)
}

pub fn get_podcast_from_title(title_: &str) -> QueryResult<Podcast> {
    use schema::podcast::dsl::*;

    let con = POOL.clone().get().unwrap();
    podcast
        .filter(title.eq(title_))
        .get_result::<Podcast>(&*con)
}

pub fn get_episode_from_uri(uri_: &str) -> QueryResult<Episode> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();
    episode.filter(uri.eq(uri_)).get_result::<Episode>(&*con)
}

pub fn remove_feed(pd: &Podcast) -> QueryResult<usize> {
    let con = POOL.clone().get().unwrap();

    con.transaction(|| -> QueryResult<usize> {
        delete_source(pd.source_id())?;
        delete_podcast(*pd.id())?;
        delete_podcast_episodes(*pd.id())
    })
}

pub fn delete_source(source_id: i32) -> QueryResult<usize> {
    use schema::source::dsl::*;

    let con = POOL.clone().get().unwrap();
    diesel::delete(source.filter(id.eq(source_id))).execute(&*con)
}

pub fn delete_podcast(podcast_id: i32) -> QueryResult<usize> {
    use schema::podcast::dsl::*;

    let con = POOL.clone().get().unwrap();
    diesel::delete(podcast.filter(id.eq(podcast_id))).execute(&*con)
}

pub fn delete_podcast_episodes(parent_id: i32) -> QueryResult<usize> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();
    diesel::delete(episode.filter(podcast_id.eq(parent_id))).execute(&*con)
}

pub fn update_none_to_played_now(parent: &Podcast) -> QueryResult<usize> {
    use schema::episode::dsl::*;

    let con = POOL.clone().get().unwrap();

    let epoch_now = Utc::now().timestamp() as i32;
    con.transaction(|| -> QueryResult<usize> {
        diesel::update(Episode::belonging_to(parent).filter(played.is_null()))
            .set(played.eq(Some(epoch_now)))
            .execute(&*con)
    })
}
