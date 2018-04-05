//! Random CRUD helper functions.

use chrono::prelude::*;
use diesel::prelude::*;

use diesel;
use diesel::dsl::exists;
use diesel::select;

use database::connection;
use errors::DataError;
use models::*;

pub fn get_sources() -> Result<Vec<Source>, DataError> {
    use schema::source::dsl::*;
    let db = connection();
    let con = db.get()?;

    source
        .order((http_etag.asc(), last_modified.asc()))
        .load::<Source>(&con)
        .map_err(From::from)
}

pub fn get_podcasts() -> Result<Vec<Podcast>, DataError> {
    use schema::podcast::dsl::*;
    let db = connection();
    let con = db.get()?;

    podcast
        .order(title.asc())
        .load::<Podcast>(&con)
        .map_err(From::from)
}

pub fn get_podcasts_filter(filter_ids: &[i32]) -> Result<Vec<Podcast>, DataError> {
    use schema::podcast::dsl::*;
    let db = connection();
    let con = db.get()?;

    podcast
        .order(title.asc())
        .filter(id.ne_any(filter_ids))
        .load::<Podcast>(&con)
        .map_err(From::from)
}

pub fn get_episodes() -> Result<Vec<Episode>, DataError> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .order(epoch.desc())
        .load::<Episode>(&con)
        .map_err(From::from)
}

pub(crate) fn get_downloaded_episodes() -> Result<Vec<EpisodeCleanerQuery>, DataError> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .select((rowid, local_uri, played))
        .filter(local_uri.is_not_null())
        .load::<EpisodeCleanerQuery>(&con)
        .map_err(From::from)
}

// pub(crate) fn get_played_episodes() -> Result<Vec<Episode>, DataError> {
//     use schema::episode::dsl::*;

//     let db = connection();
//     let con = db.get()?;
//     episode
//         .filter(played.is_not_null())
//         .load::<Episode>(&con)
//         .map_err(From::from)
// }

pub(crate) fn get_played_cleaner_episodes() -> Result<Vec<EpisodeCleanerQuery>, DataError> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .select((rowid, local_uri, played))
        .filter(played.is_not_null())
        .load::<EpisodeCleanerQuery>(&con)
        .map_err(From::from)
}

pub fn get_episode_from_rowid(ep_id: i32) -> Result<Episode, DataError> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .filter(rowid.eq(ep_id))
        .get_result::<Episode>(&con)
        .map_err(From::from)
}

pub fn get_episode_local_uri_from_id(ep_id: i32) -> Result<Option<String>, DataError> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    episode
        .filter(rowid.eq(ep_id))
        .select(local_uri)
        .get_result::<Option<String>>(&con)
        .map_err(From::from)
}

pub fn get_episodes_widgets_filter_limit(
    filter_ids: &[i32],
    limit: u32,
) -> Result<Vec<EpisodeWidgetQuery>, DataError> {
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
        .filter(episode::podcast_id.ne_any(filter_ids))
        .limit(i64::from(limit))
        .load::<EpisodeWidgetQuery>(&con)
        .map_err(From::from)
}

pub fn get_podcast_from_id(pid: i32) -> Result<Podcast, DataError> {
    use schema::podcast::dsl::*;
    let db = connection();
    let con = db.get()?;

    podcast
        .filter(id.eq(pid))
        .get_result::<Podcast>(&con)
        .map_err(From::from)
}

pub fn get_podcast_cover_from_id(pid: i32) -> Result<PodcastCoverQuery, DataError> {
    use schema::podcast::dsl::*;
    let db = connection();
    let con = db.get()?;

    podcast
        .select((id, title, image_uri))
        .filter(id.eq(pid))
        .get_result::<PodcastCoverQuery>(&con)
        .map_err(From::from)
}

pub fn get_pd_episodes(parent: &Podcast) -> Result<Vec<Episode>, DataError> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    Episode::belonging_to(parent)
        .order(epoch.desc())
        .load::<Episode>(&con)
        .map_err(From::from)
}

pub fn get_pd_episodes_count(parent: &Podcast) -> Result<i64, DataError> {
    let db = connection();
    let con = db.get()?;

    Episode::belonging_to(parent)
        .count()
        .get_result(&con)
        .map_err(From::from)
}

pub fn get_pd_episodeswidgets(parent: &Podcast) -> Result<Vec<EpisodeWidgetQuery>, DataError> {
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

pub fn get_pd_unplayed_episodes(parent: &Podcast) -> Result<Vec<Episode>, DataError> {
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
// Result<Vec<Episode>, DataError> {     use schema::episode::dsl::*;

//     let db = connection();
//     let con = db.get()?;

//     Episode::belonging_to(parent)
//         .order(epoch.desc())
//         .limit(i64::from(limit))
//         .load::<Episode>(&con)
//         .map_err(From::from)
// }

pub fn get_source_from_uri(uri_: &str) -> Result<Source, DataError> {
    use schema::source::dsl::*;
    let db = connection();
    let con = db.get()?;

    source
        .filter(uri.eq(uri_))
        .get_result::<Source>(&con)
        .map_err(From::from)
}

pub fn get_source_from_id(id_: i32) -> Result<Source, DataError> {
    use schema::source::dsl::*;
    let db = connection();
    let con = db.get()?;

    source
        .filter(id.eq(id_))
        .get_result::<Source>(&con)
        .map_err(From::from)
}

pub fn get_podcast_from_source_id(sid: i32) -> Result<Podcast, DataError> {
    use schema::podcast::dsl::*;
    let db = connection();
    let con = db.get()?;

    podcast
        .filter(source_id.eq(sid))
        .get_result::<Podcast>(&con)
        .map_err(From::from)
}

pub fn get_episode_from_pk(title_: &str, pid: i32) -> Result<Episode, DataError> {
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
) -> Result<EpisodeMinimal, DataError> {
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

pub(crate) fn remove_feed(pd: &Podcast) -> Result<(), DataError> {
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

pub fn source_exists(url: &str) -> Result<bool, DataError> {
    use schema::source::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(source.filter(uri.eq(url))))
        .get_result(&con)
        .map_err(From::from)
}

pub(crate) fn podcast_exists(source_id_: i32) -> Result<bool, DataError> {
    use schema::podcast::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(podcast.filter(source_id.eq(source_id_))))
        .get_result(&con)
        .map_err(From::from)
}

#[cfg_attr(rustfmt, rustfmt_skip)]
pub(crate) fn episode_exists(title_: &str, podcast_id_: i32) -> Result<bool, DataError> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(episode.filter(podcast_id.eq(podcast_id_)).filter(title.eq(title_))))
        .get_result(&con)
        .map_err(From::from)
}

pub(crate) fn index_new_episodes(eps: &[NewEpisode]) -> Result<(), DataError> {
    use schema::episode::dsl::*;
    let db = connection();
    let con = db.get()?;

    diesel::insert_into(episode)
        .values(eps)
        .execute(&*con)
        .map_err(From::from)
        .map(|_| ())
}

pub fn update_none_to_played_now(parent: &Podcast) -> Result<usize, DataError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use database::*;
    use pipeline::*;

    #[test]
    fn test_update_none_to_played_now() {
        truncate_db().unwrap();

        let url = "https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.\
                   com/InterceptedWithJeremyScahill";
        let source = Source::from_url(url).unwrap();
        let id = source.id();
        index_single_source(source, true).unwrap();
        let pd = get_podcast_from_source_id(id).unwrap();

        let eps_num = get_pd_unplayed_episodes(&pd).unwrap().len();
        assert_ne!(eps_num, 0);

        update_none_to_played_now(&pd).unwrap();
        let eps_num2 = get_pd_unplayed_episodes(&pd).unwrap().len();
        assert_eq!(eps_num2, 0);
    }
}
