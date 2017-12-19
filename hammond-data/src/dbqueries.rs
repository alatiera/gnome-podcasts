//! Random CRUD helper functions.

use diesel::prelude::*;
use diesel;
use models::queryables::{Episode, EpisodeViewWidgetQuery, EpisodeWidgetQuery, Podcast, Source};
use chrono::prelude::*;
use errors::*;

use database::connection;

pub fn get_sources() -> Result<Vec<Source>> {
    use schema::source::dsl::*;

    let db = connection();
    let con = db.get()?;
    Ok(source.load::<Source>(&*con)?)
}

pub fn get_podcasts() -> Result<Vec<Podcast>> {
    use schema::podcast::dsl::*;

    let db = connection();
    let con = db.get()?;
    Ok(podcast.load::<Podcast>(&*con)?)
}

pub fn get_episodes() -> Result<Vec<Episode>> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;
    Ok(episode.order(epoch.desc()).load::<Episode>(&*con)?)
}

pub fn get_downloaded_episodes() -> Result<Vec<Episode>> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;
    Ok(episode
        .filter(local_uri.is_not_null())
        .load::<Episode>(&*con)?)
}

pub fn get_played_episodes() -> Result<Vec<Episode>> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;
    Ok(episode.filter(played.is_not_null()).load::<Episode>(&*con)?)
}

pub fn get_episode_from_rowid(ep_id: i32) -> Result<Episode> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;
    Ok(episode
        .filter(rowid.eq(ep_id))
        .get_result::<Episode>(&*con)?)
}

pub fn get_episode_local_uri_from_id(ep_id: i32) -> Result<Option<String>> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;

    Ok(episode
        .filter(rowid.eq(ep_id))
        .select(local_uri)
        .get_result::<Option<String>>(&*con)?)
}

pub fn get_episodes_with_limit(limit: u32) -> Result<Vec<Episode>> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;

    Ok(episode
        .order(epoch.desc())
        .limit(i64::from(limit))
        .load::<Episode>(&*con)?)
}

pub fn get_episodes_view_widgets_with_limit(limit: u32) -> Result<Vec<EpisodeViewWidgetQuery>> {
    use schema::{episode, podcast};

    joinable!(episode -> podcast (rowid));
    allow_tables_to_appear_in_same_query!(episode, podcast);

    let db = connection();
    let con = db.get()?;

    Ok(episode::table
        .left_join(podcast::table)
        .select((
            episode::rowid,
            episode::title,
            episode::uri,
            episode::local_uri,
            episode::epoch,
            episode::length,
            episode::played,
            episode::podcast_id,
            (podcast::image_uri).nullable(),
        ))
        .order(episode::epoch.desc())
        .limit(i64::from(limit))
        .load::<EpisodeViewWidgetQuery>(&*con)?)
}

pub fn get_podcast_from_id(pid: i32) -> Result<Podcast> {
    use schema::podcast::dsl::*;

    let db = connection();
    let con = db.get()?;
    Ok(podcast.filter(id.eq(pid)).get_result::<Podcast>(&*con)?)
}

pub fn get_pd_episodes(parent: &Podcast) -> Result<Vec<Episode>> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;

    Ok(Episode::belonging_to(parent)
        .order(epoch.desc())
        .load::<Episode>(&*con)?)
}

pub fn get_pd_episodeswidgets(parent: &Podcast) -> Result<Vec<EpisodeWidgetQuery>> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;

    Ok(
        episode.select((rowid, title, uri, local_uri, epoch, length, played, podcast_id))
        .filter(podcast_id.eq(parent.id()))
        // .group_by(epoch)
        .order(epoch.desc())
        .load::<EpisodeWidgetQuery>(&*con)?,
    )
}

pub fn get_pd_unplayed_episodes(parent: &Podcast) -> Result<Vec<Episode>> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;

    Ok(Episode::belonging_to(parent)
        .filter(played.is_null())
        .order(epoch.desc())
        .load::<Episode>(&*con)?)
}

pub fn get_pd_episodes_limit(parent: &Podcast, limit: u32) -> Result<Vec<Episode>> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;

    Ok(Episode::belonging_to(parent)
        .order(epoch.desc())
        .limit(i64::from(limit))
        .load::<Episode>(&*con)?)
}

pub fn get_source_from_uri(uri_: &str) -> Result<Source> {
    use schema::source::dsl::*;

    let db = connection();
    let con = db.get()?;
    Ok(source.filter(uri.eq(uri_)).get_result::<Source>(&*con)?)
}

// pub fn get_podcast_from_title(title_: &str) -> QueryResult<Podcast> {
//     use schema::podcast::dsl::*;

//     let db = connection();
//     let con = db.get()?;
//     podcast
//         .filter(title.eq(title_))
//         .get_result::<Podcast>(&*con)
// }

pub fn get_podcast_from_source_id(sid: i32) -> Result<Podcast> {
    use schema::podcast::dsl::*;

    let db = connection();
    let con = db.get()?;
    Ok(podcast
        .filter(source_id.eq(sid))
        .get_result::<Podcast>(&*con)?)
}

pub fn get_episode_from_pk(con: &SqliteConnection, title_: &str, pid: i32) -> QueryResult<Episode> {
    use schema::episode::dsl::*;

    episode
        .filter(title.eq(title_))
        .filter(podcast_id.eq(pid))
        .get_result::<Episode>(&*con)
}

pub fn remove_feed(pd: &Podcast) -> Result<()> {
    let db = connection();
    let con = db.get()?;

    con.transaction(|| -> Result<()> {
        delete_source(&con, pd.source_id())?;
        delete_podcast(&con, *pd.id())?;
        delete_podcast_episodes(&con, *pd.id())?;
        info!("Feed removed from the Database.");
        Ok(())
    })
}

pub fn delete_source(con: &SqliteConnection, source_id: i32) -> QueryResult<usize> {
    use schema::source::dsl::*;

    diesel::delete(source.filter(id.eq(source_id))).execute(&*con)
}

pub fn delete_podcast(con: &SqliteConnection, podcast_id: i32) -> QueryResult<usize> {
    use schema::podcast::dsl::*;

    diesel::delete(podcast.filter(id.eq(podcast_id))).execute(&*con)
}

pub fn delete_podcast_episodes(con: &SqliteConnection, parent_id: i32) -> QueryResult<usize> {
    use schema::episode::dsl::*;

    diesel::delete(episode.filter(podcast_id.eq(parent_id))).execute(&*con)
}

pub fn update_none_to_played_now(parent: &Podcast) -> Result<usize> {
    use schema::episode::dsl::*;

    let db = connection();
    let con = db.get()?;

    let epoch_now = Utc::now().timestamp() as i32;
    con.transaction(|| -> Result<usize> {
        Ok(
            diesel::update(Episode::belonging_to(parent).filter(played.is_null()))
                .set(played.eq(Some(epoch_now)))
                .execute(&*con)?,
        )
    })
}
