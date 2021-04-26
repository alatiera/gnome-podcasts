// dbqueries.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Random CRUD helper functions.

use chrono::prelude::*;
use diesel::prelude::*;

use diesel::dsl::exists;
use diesel::select;

use crate::database::connection;
use crate::errors::DataError;
use crate::models::*;

pub fn get_sources() -> Result<Vec<Source>, DataError> {
    use crate::schema::source::dsl::*;
    let db = connection();
    let con = db.get()?;

    source
        .order((http_etag.asc(), last_modified.asc()))
        .load::<Source>(&con)
        .map_err(From::from)
}

pub fn get_podcasts() -> Result<Vec<Show>, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let con = db.get()?;

    shows
        .order(title.asc())
        .load::<Show>(&con)
        .map_err(From::from)
}

pub fn get_podcasts_filter(filter_ids: &[i32]) -> Result<Vec<Show>, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let con = db.get()?;

    shows
        .order(title.asc())
        .filter(id.ne_all(filter_ids))
        .load::<Show>(&con)
        .map_err(From::from)
}

pub fn get_episodes() -> Result<Vec<Episode>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    episodes
        .order(epoch.desc())
        .load::<Episode>(&con)
        .map_err(From::from)
}

pub(crate) fn get_downloaded_episodes() -> Result<Vec<EpisodeCleanerModel>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    episodes
        .select((rowid, local_uri, played))
        .filter(local_uri.is_not_null())
        .load::<EpisodeCleanerModel>(&con)
        .map_err(From::from)
}

// pub(crate) fn get_played_episodes() -> Result<Vec<Episode>, DataError> {
//     use schema::episodes::dsl::*;

//     let db = connection();
//     let con = db.get()?;
//     episodes
//         .filter(played.is_not_null())
//         .load::<Episode>(&con)
//         .map_err(From::from)
// }

pub(crate) fn get_played_cleaner_episodes() -> Result<Vec<EpisodeCleanerModel>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    episodes
        .select((rowid, local_uri, played))
        .filter(played.is_not_null())
        .load::<EpisodeCleanerModel>(&con)
        .map_err(From::from)
}

pub fn get_episode_from_rowid(ep_id: i32) -> Result<Episode, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    episodes
        .filter(rowid.eq(ep_id))
        .get_result::<Episode>(&con)
        .map_err(From::from)
}

pub fn get_episode_widget_from_rowid(ep_id: i32) -> Result<EpisodeWidgetModel, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    episodes
        .select((
            rowid,
            title,
            uri,
            local_uri,
            epoch,
            length,
            duration,
            played,
            play_position,
            show_id,
        ))
        .filter(rowid.eq(ep_id))
        .get_result::<EpisodeWidgetModel>(&con)
        .map_err(From::from)
}

pub fn get_episode_local_uri_from_id(ep_id: i32) -> Result<Option<String>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    episodes
        .filter(rowid.eq(ep_id))
        .select(local_uri)
        .get_result::<Option<String>>(&con)
        .map_err(From::from)
}

pub fn get_episodes_widgets_filter_limit(
    filter_ids: &[i32],
    limit: u32,
) -> Result<Vec<EpisodeWidgetModel>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;
    let columns = (
        rowid,
        title,
        uri,
        local_uri,
        epoch,
        length,
        duration,
        played,
        play_position,
        show_id,
    );

    episodes
        .select(columns)
        .order(epoch.desc())
        .filter(show_id.ne_all(filter_ids))
        .limit(i64::from(limit))
        .load::<EpisodeWidgetModel>(&con)
        .map_err(From::from)
}

pub fn get_podcast_from_id(pid: i32) -> Result<Show, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let con = db.get()?;

    shows
        .filter(id.eq(pid))
        .get_result::<Show>(&con)
        .map_err(From::from)
}

pub fn get_podcast_cover_from_id(pid: i32) -> Result<ShowCoverModel, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let con = db.get()?;

    shows
        .select((id, title, image_uri, image_uri_hash, image_cached))
        .filter(id.eq(pid))
        .get_result::<ShowCoverModel>(&con)
        .map_err(From::from)
}

pub fn get_pd_episodes(parent: &Show) -> Result<Vec<Episode>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    Episode::belonging_to(parent)
        .order(epoch.desc())
        .load::<Episode>(&con)
        .map_err(From::from)
}

pub fn get_pd_episodes_count(parent: &Show) -> Result<i64, DataError> {
    let db = connection();
    let con = db.get()?;

    Episode::belonging_to(parent)
        .count()
        .get_result(&con)
        .map_err(From::from)
}

pub fn get_pd_episodeswidgets(parent: &Show) -> Result<Vec<EpisodeWidgetModel>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;
    let columns = (
        rowid,
        title,
        uri,
        local_uri,
        epoch,
        length,
        duration,
        played,
        play_position,
        show_id,
    );

    episodes
        .select(columns)
        .filter(show_id.eq(parent.id()))
        .order(epoch.desc())
        .load::<EpisodeWidgetModel>(&con)
        .map_err(From::from)
}

pub fn get_pd_unplayed_episodes(parent: &Show) -> Result<Vec<Episode>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    Episode::belonging_to(parent)
        .filter(played.is_null())
        .order(epoch.desc())
        .load::<Episode>(&con)
        .map_err(From::from)
}

// pub(crate) fn get_pd_episodes_limit(parent: &Show, limit: u32) ->
// Result<Vec<Episode>, DataError> {     use schema::episodes::dsl::*;

//     let db = connection();
//     let con = db.get()?;

//     Episode::belonging_to(parent)
//         .order(epoch.desc())
//         .limit(i64::from(limit))
//         .load::<Episode>(&con)
//         .map_err(From::from)
// }

pub fn get_source_from_uri(uri_: &str) -> Result<Source, DataError> {
    use crate::schema::source::dsl::*;
    let db = connection();
    let con = db.get()?;

    source
        .filter(uri.eq(uri_))
        .get_result::<Source>(&con)
        .map_err(From::from)
}

pub fn get_source_from_id(id_: i32) -> Result<Source, DataError> {
    use crate::schema::source::dsl::*;
    let db = connection();
    let con = db.get()?;

    source
        .filter(id.eq(id_))
        .get_result::<Source>(&con)
        .map_err(From::from)
}

pub fn get_podcast_from_source_id(sid: i32) -> Result<Show, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let con = db.get()?;

    shows
        .filter(source_id.eq(sid))
        .get_result::<Show>(&con)
        .map_err(From::from)
}

pub fn get_episode_from_pk(title_: &str, pid: i32) -> Result<Episode, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    episodes
        .filter(title.eq(title_))
        .filter(show_id.eq(pid))
        .get_result::<Episode>(&con)
        .map_err(From::from)
}

pub(crate) fn get_episode_minimal_from_pk(
    title_: &str,
    pid: i32,
) -> Result<EpisodeMinimal, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    episodes
        .select((
            rowid,
            title,
            uri,
            epoch,
            length,
            duration,
            play_position,
            guid,
            show_id,
        ))
        .filter(title.eq(title_))
        .filter(show_id.eq(pid))
        .get_result::<EpisodeMinimal>(&con)
        .map_err(From::from)
}

#[cfg(test)]
pub(crate) fn get_episode_cleaner_from_pk(
    title_: &str,
    pid: i32,
) -> Result<EpisodeCleanerModel, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    episodes
        .select((rowid, local_uri, played))
        .filter(title.eq(title_))
        .filter(show_id.eq(pid))
        .get_result::<EpisodeCleanerModel>(&con)
        .map_err(From::from)
}

pub(crate) fn remove_feed(pd: &Show) -> Result<(), DataError> {
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
    use crate::schema::source::dsl::*;

    diesel::delete(source.filter(id.eq(source_id))).execute(con)
}

fn delete_podcast(con: &SqliteConnection, show_id: i32) -> QueryResult<usize> {
    use crate::schema::shows::dsl::*;

    diesel::delete(shows.filter(id.eq(show_id))).execute(con)
}

fn delete_podcast_episodes(con: &SqliteConnection, parent_id: i32) -> QueryResult<usize> {
    use crate::schema::episodes::dsl::*;

    diesel::delete(episodes.filter(show_id.eq(parent_id))).execute(con)
}

pub fn source_exists(url: &str) -> Result<bool, DataError> {
    use crate::schema::source::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(source.filter(uri.eq(url))))
        .get_result(&con)
        .map_err(From::from)
}

pub(crate) fn podcast_exists(source_id_: i32) -> Result<bool, DataError> {
    use crate::schema::shows::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(shows.filter(source_id.eq(source_id_))))
        .get_result(&con)
        .map_err(From::from)
}

pub(crate) fn episode_exists(title_: &str, show_id_: i32) -> Result<bool, DataError> {
    use crate::schema::episodes::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(
        episodes
            .filter(show_id.eq(show_id_))
            .filter(title.eq(title_)),
    ))
    .get_result(&con)
    .map_err(From::from)
}

/// Check if the `episodes table contains any rows
///
/// Return true if `episodes` table is populated.
pub fn is_episodes_populated(filter_show_ids: &[i32]) -> Result<bool, DataError> {
    use crate::schema::episodes::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(episodes.filter(show_id.ne_all(filter_show_ids))))
        .get_result(&con)
        .map_err(From::from)
}

/// Check if the `shows` table contains any rows
///
/// Return true if `shows` table is populated.
pub fn is_podcasts_populated(filter_ids: &[i32]) -> Result<bool, DataError> {
    use crate::schema::shows::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(shows.filter(id.ne_all(filter_ids))))
        .get_result(&con)
        .map_err(From::from)
}

/// Check if the `source` table contains any rows
///
/// Return true if `source` table is populated.
pub fn is_source_populated(filter_ids: &[i32]) -> Result<bool, DataError> {
    use crate::schema::source::dsl::*;

    let db = connection();
    let con = db.get()?;

    select(exists(source.filter(id.ne_all(filter_ids))))
        .get_result(&con)
        .map_err(From::from)
}

pub(crate) fn index_new_episodes(eps: &[NewEpisode]) -> Result<(), DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let con = db.get()?;

    diesel::insert_into(episodes)
        .values(eps)
        .execute(&*con)
        .map_err(From::from)
        .map(|_| ())
}

pub fn update_none_to_played_now(parent: &Show) -> Result<usize, DataError> {
    use crate::schema::episodes::dsl::*;
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
    use crate::database::*;
    use crate::pipeline::pipeline;
    use anyhow::Result;

    #[test]
    fn test_update_none_to_played_now() -> Result<()> {
        truncate_db()?;

        let url = "https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.\
                   com/InterceptedWithJeremyScahill";
        let source = Source::from_url(url)?;
        let id = source.id();
        let mut rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(vec![source]));
        let pd = get_podcast_from_source_id(id)?;

        let eps_num = get_pd_unplayed_episodes(&pd)?.len();
        assert_ne!(eps_num, 0);

        update_none_to_played_now(&pd)?;
        let eps_num2 = get_pd_unplayed_episodes(&pd)?.len();
        assert_eq!(eps_num2, 0);
        Ok(())
    }
}
