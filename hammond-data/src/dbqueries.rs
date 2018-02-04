//! Random CRUD helper functions.

use chrono::prelude::*;
use diesel::prelude::*;

use diesel;
use diesel::dsl::exists;
use diesel::select;

use database::connection;
use errors::DataError;
use models::*;

// Feel free to open a Merge request that manually replaces Result<T> if you feel bored.
use std::result;
type DatabaseResult<T> = result::Result<T, DataError>;

pub fn get_sources() -> DatabaseResult<Vec<Source>> {
    use schema::source::dsl::*;
    let db = connection();
    let con = db.get()?;

    source
        .order((http_etag.asc(), last_modified.asc()))
        .load::<Source>(&con)
        .map_err(From::from)
}

pub fn get_podcasts() -> DatabaseResult<Vec<Podcast>> {
    use schema::podcast::dsl::*;
    let db = connection();
    let con = db.get()?;

    podcast
        .order(title.asc())
        .load::<Podcast>(&con)
        .map_err(From::from)
}

pub fn get_episodes() -> DatabaseResult<Vec<Episode>> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .order(epoch.desc())
        .load::<Episode>(&con)
        .map_err(From::from)
}

pub(crate) fn get_downloaded_episodes() -> DatabaseResult<Vec<EpisodeCleanerQuery>> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .select((rowid, local_uri, played))
        .filter(local_uri.is_not_null())
        .load::<EpisodeCleanerQuery>(&con)
        .map_err(From::from)
}

// pub(crate) fn get_played_episodes() -> DatabaseResult<Vec<Episode>> {
//     use schema::episode::dsl::*;

//     let db = connection();
//     let con = db.get()?;
//     episode
//         .filter(played.is_not_null())
//         .load::<Episode>(&con)
//         .map_err(From::from)
// }

pub(crate) fn get_played_cleaner_episodes() -> DatabaseResult<Vec<EpisodeCleanerQuery>> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .select((rowid, local_uri, played))
        .filter(played.is_not_null())
        .load::<EpisodeCleanerQuery>(&con)
        .map_err(From::from)
}

pub fn get_episode_from_rowid(ep_id: i32) -> DatabaseResult<Episode> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .filter(rowid.eq(ep_id))
        .get_result::<Episode>(&con)
        .map_err(From::from)
}

pub fn get_episode_local_uri_from_id(ep_id: i32) -> DatabaseResult<Option<String>> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .filter(rowid.eq(ep_id))
        .select(local_uri)
        .get_result::<Option<String>>(&con)
        .map_err(From::from)
}

pub fn get_episodes_widgets_with_limit(limit: u32) -> DatabaseResult<Vec<EpisodeWidgetQuery>> {
    use schema::episode;
    let db = connection();
    let con = db.get()?;

    episode::table
        .select((
            episode::rowid,
            episode::title,
            episode::uri,
            episode::local_uri,
            episode::epoch,
            episode::length,
            episode::duration,
            episode::played,
            episode::podcast_id,
        ))
        .order(episode::epoch.desc())
        .limit(i64::from(limit))
        .load::<EpisodeWidgetQuery>(&con)
        .map_err(From::from)
}

pub fn get_podcast_from_id(pid: i32) -> DatabaseResult<Podcast> {
    use schema::podcast::dsl::*;
    let db = connection();
    let con = db.get()?;

    podcast
        .filter(id.eq(pid))
        .get_result::<Podcast>(&con)
        .map_err(From::from)
}

pub fn get_podcast_cover_from_id(pid: i32) -> DatabaseResult<PodcastCoverQuery> {
    use schema::podcast::dsl::*;
    let db = connection();
    let con = db.get()?;

    podcast
        .select((id, title, image_uri))
        .filter(id.eq(pid))
        .get_result::<PodcastCoverQuery>(&con)
        .map_err(From::from)
}

pub fn get_pd_episodes(parent: &Podcast) -> DatabaseResult<Vec<Episode>> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    Episode::belonging_to(parent)
        .order(epoch.desc())
        .load::<Episode>(&con)
        .map_err(From::from)
}

pub fn get_pd_episodeswidgets(parent: &Podcast) -> DatabaseResult<Vec<EpisodeWidgetQuery>> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode.select((rowid, title, uri, local_uri, epoch, length, duration, played, podcast_id))
        .filter(podcast_id.eq(parent.id()))
        // .group_by(epoch)
        .order(epoch.desc())
        .load::<EpisodeWidgetQuery>(&con)
        .map_err(From::from)
}

pub fn get_pd_unplayed_episodes(parent: &Podcast) -> DatabaseResult<Vec<Episode>> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    Episode::belonging_to(parent)
        .filter(played.is_null())
        .order(epoch.desc())
        .load::<Episode>(&con)
        .map_err(From::from)
}

// pub(crate) fn get_pd_episodes_limit(parent: &Podcast, limit: u32) ->
// DatabaseResult<Vec<Episode>> {     use schema::episode::dsl::*;

//     let db = connection();
//     let con = db.get()?;

//     Episode::belonging_to(parent)
//         .order(epoch.desc())
//         .limit(i64::from(limit))
//         .load::<Episode>(&con)
//         .map_err(From::from)
// }

pub fn get_source_from_uri(uri_: &str) -> DatabaseResult<Source> {
    use schema::source::dsl::*;
    let db = connection();
    let con = db.get()?;

    source
        .filter(uri.eq(uri_))
        .get_result::<Source>(&con)
        .map_err(From::from)
}

pub fn get_source_from_id(id_: i32) -> DatabaseResult<Source> {
    use schema::source::dsl::*;
    let db = connection();
    let con = db.get()?;

    source
        .filter(id.eq(id_))
        .get_result::<Source>(&con)
        .map_err(From::from)
}

pub fn get_podcast_from_source_id(sid: i32) -> DatabaseResult<Podcast> {
    use schema::podcast::dsl::*;
    let db = connection();
    let con = db.get()?;

    podcast
        .filter(source_id.eq(sid))
        .get_result::<Podcast>(&con)
        .map_err(From::from)
}

pub fn get_episode_from_pk(title_: &str, pid: i32) -> DatabaseResult<Episode> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .filter(title.eq(title_))
        .filter(podcast_id.eq(pid))
        .get_result::<Episode>(&con)
        .map_err(From::from)
}

pub(crate) fn get_episode_minimal_from_pk(
    title_: &str,
    pid: i32,
) -> DatabaseResult<EpisodeMinimal> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .select((rowid, title, uri, epoch, duration, guid, podcast_id))
        .filter(title.eq(title_))
        .filter(podcast_id.eq(pid))
        .get_result::<EpisodeMinimal>(&con)
        .map_err(From::from)
}

pub(crate) fn remove_feed(pd: &Podcast) -> DatabaseResult<()> {
    let db = connection();
    let con = db.get()?;

    con.transaction(|| {
        delete_source(&con, pd.source_id())?;
        delete_podcast(&con, pd.id())?;
        delete_podcast_episodes(&con, pd.id())?;
        info!("Feed removed from the Database.");
        Ok(())
    })
}

fn delete_source(con: &SqliteConnection, source_id: i32) -> QueryResult<usize> {
    use schema::source::dsl::*;

    diesel::delete(source.filter(id.eq(source_id))).execute(con)
}

fn delete_podcast(con: &SqliteConnection, podcast_id: i32) -> QueryResult<usize> {
    use schema::podcast::dsl::*;

    diesel::delete(podcast.filter(id.eq(podcast_id))).execute(con)
}

fn delete_podcast_episodes(con: &SqliteConnection, parent_id: i32) -> QueryResult<usize> {
    use schema::episode::dsl::*;

    diesel::delete(episode.filter(podcast_id.eq(parent_id))).execute(con)
}

pub fn source_exists(url: &str) -> DatabaseResult<bool> {
    use schema::source::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(source.filter(uri.eq(url))))
        .get_result(&con)
        .map_err(From::from)
}

pub(crate) fn podcast_exists(source_id_: i32) -> DatabaseResult<bool> {
    use schema::podcast::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(podcast.filter(source_id.eq(source_id_))))
        .get_result(&con)
        .map_err(From::from)
}

#[cfg_attr(rustfmt, rustfmt_skip)]
pub(crate) fn episode_exists(title_: &str, podcast_id_: i32) -> DatabaseResult<bool> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(episode.filter(podcast_id.eq(podcast_id_)).filter(title.eq(title_))))
        .get_result(&con)
        .map_err(From::from)
}

pub(crate) fn index_new_episodes(eps: &[NewEpisode]) -> DatabaseResult<()> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    diesel::insert_into(episode)
        .values(eps)
        .execute(&*con)
        .map_err(From::from)
        .map(|_| ())
}

pub fn update_none_to_played_now(parent: &Podcast) -> DatabaseResult<usize> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    let epoch_now = Utc::now().timestamp() as i32;
    con.transaction(|| {
        diesel::update(Episode::belonging_to(parent).filter(played.is_null()))
            .set(played.eq(Some(epoch_now)))
            .execute(&con)
            .map_err(From::from)
    })
}
