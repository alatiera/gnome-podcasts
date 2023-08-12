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
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::select;
use std::collections::HashMap;

use crate::database::connection;
use crate::errors::DataError;
use crate::models::*;

pub fn get_sources() -> Result<Vec<Source>, DataError> {
    use crate::schema::source::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    source
        .order((http_etag.asc(), last_modified.asc()))
        .load::<Source>(&mut con)
        .map_err(From::from)
}

pub fn get_podcasts() -> Result<Vec<Show>, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    shows
        .order(title.asc())
        .load::<Show>(&mut con)
        .map_err(From::from)
}

pub fn get_podcasts_filter(filter_ids: &[ShowId]) -> Result<Vec<Show>, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    shows
        .order(title.asc())
        .filter(id.ne_all(filter_ids))
        .load::<Show>(&mut con)
        .map_err(From::from)
}

pub fn get_episodes() -> Result<Vec<Episode>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .order(epoch.desc())
        .load::<Episode>(&mut con)
        .map_err(From::from)
}

fn get_episodes_unsorted() -> Result<Vec<Episode>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes.load::<Episode>(&mut con).map_err(From::from)
}

pub fn get_episodes_by_urls_or_guids(
    urls: Vec<&str>,
    guids: Vec<&str>,
) -> Result<Vec<Episode>, DataError> {
    let eps = get_episodes_unsorted()?;
    let filtered: Vec<Episode> = eps
        .into_iter()
        .filter(|ep| {
            ep.uri().map(|uri| urls.contains(&uri)).unwrap_or_default()
                || ep
                    .guid()
                    .map(|guid| guids.contains(&guid))
                    .unwrap_or_default()
        })
        .collect();
    Ok(filtered)
}

pub(crate) fn get_downloaded_episodes() -> Result<Vec<EpisodeCleanerModel>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .select(EpisodeCleanerModel::as_select())
        .filter(local_uri.is_not_null())
        .load::<EpisodeCleanerModel>(&mut con)
        .map_err(From::from)
}

pub(crate) fn get_played_cleaner_episodes() -> Result<Vec<EpisodeCleanerModel>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .select(EpisodeCleanerModel::as_select())
        .filter(played.is_not_null())
        .load::<EpisodeCleanerModel>(&mut con)
        .map_err(From::from)
}

pub fn get_episode_from_id(ep_id: EpisodeId) -> Result<Episode, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .filter(id.eq(ep_id))
        .get_result::<Episode>(&mut con)
        .map_err(From::from)
}

pub fn get_episode_widget_from_id(ep_id: EpisodeId) -> Result<EpisodeWidgetModel, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .select(EpisodeWidgetModel::as_select())
        .filter(id.eq(ep_id))
        .get_result::<EpisodeWidgetModel>(&mut con)
        .map_err(From::from)
}

pub fn get_episode_local_uri_from_id(ep_id: EpisodeId) -> Result<Option<String>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .filter(id.eq(ep_id))
        .select(local_uri)
        .get_result::<Option<String>>(&mut con)
        .map_err(From::from)
}

pub fn get_episodes_widgets_filter_limit(
    filter_ids: &[ShowId],
    limit: u32,
) -> Result<Vec<EpisodeWidgetModel>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .select(EpisodeWidgetModel::as_select())
        .order(epoch.desc())
        .filter(show_id.ne_all(filter_ids))
        .limit(i64::from(limit))
        .load::<EpisodeWidgetModel>(&mut con)
        .map_err(From::from)
}

pub fn get_podcast_from_id(pid: ShowId) -> Result<Show, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    shows
        .filter(id.eq(pid))
        .get_result::<Show>(&mut con)
        .map_err(From::from)
}

pub fn get_podcast_cover_from_id(pid: ShowId) -> Result<ShowCoverModel, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    shows
        .select(ShowCoverModel::as_select())
        .filter(id.eq(pid))
        .get_result::<ShowCoverModel>(&mut con)
        .map_err(From::from)
}

pub fn get_pd_episodes(parent: &Show) -> Result<Vec<Episode>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    Episode::belonging_to(parent)
        .order(epoch.desc())
        .load::<Episode>(&mut con)
        .map_err(From::from)
}

pub fn get_pd_episodes_count(parent: &Show) -> Result<i64, DataError> {
    let db = connection();
    let mut con = db.get()?;

    Episode::belonging_to(parent)
        .count()
        .get_result(&mut con)
        .map_err(From::from)
}

pub fn get_pd_episodeswidgets(parent: &Show) -> Result<Vec<EpisodeWidgetModel>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .select(EpisodeWidgetModel::as_select())
        .filter(show_id.eq(parent.id()))
        .order(epoch.desc())
        .load::<EpisodeWidgetModel>(&mut con)
        .map_err(From::from)
}

pub fn get_pd_unplayed_episodes(parent: &Show) -> Result<Vec<Episode>, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    Episode::belonging_to(parent)
        .filter(played.is_null())
        .order(epoch.desc())
        .load::<Episode>(&mut con)
        .map_err(From::from)
}

pub fn get_source_from_uri(uri_: &str) -> Result<Source, DataError> {
    use crate::schema::source::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    source
        .filter(uri.eq(uri_))
        .get_result::<Source>(&mut con)
        .map_err(From::from)
}

pub fn get_source_from_id(id_: SourceId) -> Result<Source, DataError> {
    use crate::schema::source::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    source
        .filter(id.eq(id_))
        .get_result::<Source>(&mut con)
        .map_err(From::from)
}

pub fn get_podcast_from_source_id(sid: SourceId) -> Result<Show, DataError> {
    use crate::schema::shows::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    shows
        .filter(source_id.eq(sid))
        .get_result::<Show>(&mut con)
        .map_err(From::from)
}

pub fn get_podcast_from_uri(uri_: &str) -> Result<(Source, Show), DataError> {
    use crate::schema::shows::dsl::*;
    use crate::schema::source::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    source
        .inner_join(shows)
        .filter(uri.eq(uri_))
        .get_result::<(Source, Show)>(&mut con)
        .map_err(From::from)
}

fn get_episode_from_title(title_: &str, pid: ShowId) -> Result<Episode, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .filter(title.eq(title_))
        .filter(show_id.eq(pid))
        .get_result::<Episode>(&mut con)
        .map_err(From::from)
}

pub fn get_episode(guid: Option<&str>, title: &str, show_id: ShowId) -> Result<Episode, DataError> {
    if guid.is_some() {
        get_episode_from_guid(guid, show_id)
    } else {
        get_episode_from_title(title, show_id)
    }
}

fn get_episode_from_guid(guid_: Option<&str>, pid: ShowId) -> Result<Episode, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .filter(guid.eq(guid_))
        .filter(show_id.eq(pid))
        .get_result::<Episode>(&mut con)
        .map_err(From::from)
}

fn get_episode_minimal_from_title(title_: &str, pid: ShowId) -> Result<EpisodeMinimal, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .select(EpisodeMinimal::as_select())
        .filter(title.eq(title_))
        .filter(show_id.eq(pid))
        .get_result::<EpisodeMinimal>(&mut con)
        .map_err(From::from)
}

pub(crate) fn get_episode_minimal(
    guid: Option<&str>,
    title: &str,
    show_id: ShowId,
) -> Result<EpisodeMinimal, DataError> {
    if guid.is_some() {
        get_episode_minimal_from_guid(&guid, show_id)
    } else {
        get_episode_minimal_from_title(title, show_id)
    }
}

fn get_episode_minimal_from_guid(
    guid_: &Option<&str>,
    pid: ShowId,
) -> Result<EpisodeMinimal, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .select(EpisodeMinimal::as_select())
        .filter(guid.eq(guid_))
        .filter(show_id.eq(pid))
        .get_result::<EpisodeMinimal>(&mut con)
        .map_err(From::from)
}

#[cfg(test)]
pub(crate) fn get_episode_cleaner_from_title(
    title_: &str,
    pid: ShowId,
) -> Result<EpisodeCleanerModel, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    episodes
        .select(EpisodeCleanerModel::as_select())
        .filter(title.eq(title_))
        .filter(show_id.eq(pid))
        .get_result::<EpisodeCleanerModel>(&mut con)
        .map_err(From::from)
}

pub(crate) fn remove_feed(pd: &Show) -> Result<(), DataError> {
    let db = connection();
    let mut con = db.get()?;

    con.transaction(|conn| {
        delete_source(conn, pd.source_id())?;
        delete_podcast(conn, pd.id())?;
        delete_podcast_episodes(conn, pd.id())?;
        info!("Feed removed from the Database.");
        Ok(())
    })
}

/// use utils::delete_show if the podcast was fully imported
pub fn remove_source(source: &Source) -> Result<(), DataError> {
    let db = connection();
    let mut con = db.get()?;

    delete_source(&mut con, source.id())
        .map(|_| ())
        .map_err(From::from)
}

pub(crate) fn remove_feed_by_uri(uri: &str) -> Result<(), DataError> {
    use crate::schema::shows::dsl::*;
    let source = get_source_from_uri(uri)?;
    let show = {
        let db = connection();
        let mut con = db.get()?;

        shows
            .filter(source_id.eq(source.id()))
            .limit(1)
            .get_result::<Show>(&mut con)?
    };

    remove_feed(&show)?;
    Ok(())
}

fn delete_source(con: &mut SqliteConnection, source_id: SourceId) -> QueryResult<usize> {
    use crate::schema::source::dsl::*;

    diesel::delete(source.filter(id.eq(source_id))).execute(con)
}

fn delete_podcast(con: &mut SqliteConnection, show_id: ShowId) -> QueryResult<usize> {
    use crate::schema::shows::dsl::*;

    diesel::delete(shows.filter(id.eq(show_id))).execute(con)
}

fn delete_podcast_episodes(con: &mut SqliteConnection, parent_id: ShowId) -> QueryResult<usize> {
    use crate::schema::episodes::dsl::*;

    diesel::delete(episodes.filter(show_id.eq(parent_id))).execute(con)
}

pub fn source_exists(url: &str) -> Result<bool, DataError> {
    use crate::schema::source::dsl::*;

    let db = connection();
    let mut con = db.get()?;

    select(exists(source.filter(uri.eq(url))))
        .get_result(&mut con)
        .map_err(From::from)
}

pub(crate) fn podcast_exists(source_id_: SourceId) -> Result<bool, DataError> {
    use crate::schema::shows::dsl::*;

    let db = connection();
    let mut con = db.get()?;

    select(exists(shows.filter(source_id.eq(source_id_))))
        .get_result(&mut con)
        .map_err(From::from)
}

pub(crate) fn episode_exists(
    guid_: Option<&str>,
    title_: &str,
    show_id_: ShowId,
) -> Result<bool, DataError> {
    use crate::schema::episodes::dsl::*;

    let db = connection();
    let mut con = db.get()?;

    if guid_.is_some() {
        return select(exists(
            episodes.filter(show_id.eq(show_id_)).filter(guid.eq(guid_)),
        ))
        .get_result(&mut con)
        .map_err(From::from);
    }

    select(exists(
        episodes
            .filter(show_id.eq(show_id_))
            .filter(title.eq(title_)),
    ))
    .get_result(&mut con)
    .map_err(From::from)
}

/// Check if the `episodes table contains any rows
///
/// Return true if `episodes` table is populated.
pub fn is_episodes_populated(filter_show_ids: &[ShowId]) -> Result<bool, DataError> {
    use crate::schema::episodes::dsl::*;

    let db = connection();
    let mut con = db.get()?;

    select(exists(episodes.filter(show_id.ne_all(filter_show_ids))))
        .get_result(&mut con)
        .map_err(From::from)
}

/// Check if the `shows` table contains any rows
///
/// Return true if `shows` table is populated.
pub fn is_podcasts_populated(filter_ids: &[ShowId]) -> Result<bool, DataError> {
    use crate::schema::shows::dsl::*;

    let db = connection();
    let mut con = db.get()?;

    select(exists(shows.filter(id.ne_all(filter_ids))))
        .get_result(&mut con)
        .map_err(From::from)
}

/// Check if the `source` table contains any rows
///
/// Return true if `source` table is populated.
pub fn is_source_populated(filter_ids: &[ShowId]) -> Result<bool, DataError> {
    use crate::schema::source::dsl::*;

    let db = connection();
    let mut con = db.get()?;

    select(exists(source.filter(id.ne_all(filter_ids))))
        .get_result(&mut con)
        .map_err(From::from)
}

pub(crate) fn index_new_episodes(eps: &[NewEpisode]) -> Result<(), DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    diesel::insert_into(episodes)
        .values(eps)
        .execute(&mut con)
        .map_err(From::from)
        .map(|_| ())
}

pub fn update_none_to_played_now(parent: &Show) -> Result<usize, DataError> {
    use crate::schema::episodes::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    let epoch_now = Utc::now().naive_utc();
    con.transaction(|conn| {
        diesel::update(Episode::belonging_to(parent).filter(played.is_null()))
            .set(played.eq(Some(epoch_now)))
            .execute(conn)
            .map_err(From::from)
    })
}

fn get_discovery_settings_err() -> Result<HashMap<String, bool>, DataError> {
    use crate::schema::discovery_settings::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    discovery_settings
        .load::<DiscoverySetting>(&mut con)
        .map(|v| {
            v.into_iter()
                .map(|ds| (ds.platform_id, ds.enabled))
                .collect()
        })
        .map_err(From::from)
}

pub fn get_discovery_settings() -> HashMap<String, bool> {
    get_discovery_settings_err().unwrap_or_default()
}

pub fn set_discovery_setting(pid: &str, value: bool) -> Result<(), DataError> {
    use crate::schema::discovery_settings::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    let item = DiscoverySetting {
        platform_id: pid.to_string(),
        enabled: value,
    };

    diesel::insert_into(discovery_settings)
        .values(&item)
        .on_conflict(platform_id)
        .do_update()
        .set(&item)
        .execute(&mut con)
        .map(|_| ())
        .map_err(From::from)
}

pub fn update_episodes(eps: Vec<Episode>) -> Result<(), DataError> {
    let db = connection();
    let mut tempdb = db.get()?;

    for e in eps {
        let r: Result<Episode, DataError> =
            e.save_changes::<Episode>(&mut tempdb).map_err(From::from);
        r?;
    }
    Ok(())
}

// All the data required to make a delta sync
#[allow(clippy::type_complexity)]
pub(crate) fn get_sync_delta_data() -> Result<
    (
        Vec<(crate::sync::Show, Option<Source>, Option<Show>)>,
        Vec<(crate::sync::Episode, Episode, Show, Source)>,
    ),
    DataError,
> {
    use crate::schema::episodes::dsl::*;
    use crate::schema::episodes_sync::dsl::*;
    use crate::schema::shows::dsl::*;
    use crate::schema::shows_sync::dsl::uri as shows_uri;
    use crate::schema::shows_sync::dsl::*;
    use crate::schema::source::dsl::id as sources_id;
    use crate::schema::source::dsl::uri as source_uri;
    use crate::schema::source::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    let shows_data = shows_sync
        .left_join(source.on(source_uri.eq(shows_uri)))
        .left_join(shows.on(sources_id.eq(source_id)))
        .load(&mut con)?;

    let ep_data = episodes_sync
        .inner_join(episodes.on(crate::schema::episodes::dsl::id.eq(ep_id)))
        .inner_join(shows.on(crate::schema::shows::dsl::id.eq(show_id)))
        .inner_join(source.on(crate::schema::source::dsl::id.eq(source_id)))
        .load(&mut con)?;

    Ok((shows_data, ep_data))
}

// Data required to apply episode changes downloaded from nextcloud.
// pub(crate) fn get_episode_sync_update_data(ep_urls: Vec<&str>) -> Result<(HashMap<String, (Episode)>), DataError> {

pub(crate) fn get_podcast_ids_to_uris() -> Result<HashMap<ShowId, String>, DataError> {
    use crate::schema::shows::dsl::*;
    use crate::schema::source::dsl::*;
    let db = connection();
    let mut con = db.get()?;

    let pairs: Vec<(ShowId, String)> = shows
        .inner_join(source)
        .select((
            crate::schema::shows::dsl::id,
            crate::schema::source::dsl::uri,
        ))
        .load(&mut con)?;

    let shows_data = HashMap::from_iter(pairs);
    Ok(shows_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::truncate_db;
    use crate::pipeline::pipeline;
    use crate::utils::get_feed;
    use anyhow::Result;

    #[test]
    fn test_update_none_to_played_now() -> Result<()> {
        truncate_db()?;

        let url = "https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.\
                   com/InterceptedWithJeremyScahill";
        let source = Source::from_url(url)?;
        let id = source.id();
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(vec![source]))?;
        let pd = get_podcast_from_source_id(id)?;

        let eps_num = get_pd_unplayed_episodes(&pd)?.len();
        assert_ne!(eps_num, 0);

        update_none_to_played_now(&pd)?;
        let eps_num2 = get_pd_unplayed_episodes(&pd)?.len();
        assert_eq!(eps_num2, 0);
        Ok(())
    }

    #[test]
    fn test_episode_exists() -> Result<()> {
        truncate_db()?;
        const TEST_SHOW_ID: ShowId = ShowId(1);
        const TEST_SOURCE_ID: SourceId = SourceId(1);

        let path = "tests/feeds/2024-03-13-ndr.xml";
        let feed = get_feed(path, TEST_SOURCE_ID);
        feed.index()?;

        // only title given
        assert!(episode_exists(None, "Nachrichten", TEST_SHOW_ID)?);
        assert!(get_episode(None, "Nachrichten", TEST_SHOW_ID).is_ok());
        assert!(get_episode_minimal(None, "Nachrichten", TEST_SHOW_ID).is_ok());

        // only GUID matches, title is different
        assert!(episode_exists(
            Some("AU-20230622-0747-4100-A"),
            "wrong",
            TEST_SHOW_ID
        )?);
        assert!(get_episode(Some("AU-20230622-0747-4100-A"), "wrong", TEST_SHOW_ID).is_ok());
        assert!(
            get_episode_minimal(Some("AU-20230622-0747-4100-A"), "wrong", TEST_SHOW_ID).is_ok()
        );

        // wrong guid
        // Should not find, different guid = assume it's a different episode
        assert!(!episode_exists(Some("wrong"), "Nachrichten", TEST_SHOW_ID)?);
        assert!(!get_episode(Some("wrong"), "Nachrichten", TEST_SHOW_ID).is_ok());
        assert!(!get_episode_minimal(Some("wrong"), "Nachrichten", TEST_SHOW_ID).is_ok());

        // no result
        assert!(!episode_exists(None, "wrong", TEST_SHOW_ID)?);
        assert!(!get_episode(None, "wrong", TEST_SHOW_ID).is_ok());
        assert!(!get_episode_minimal(None, "wrong", TEST_SHOW_ID).is_ok());
        Ok(())
    }

    fn test_get_sync_delta_data() -> Result<()> {
        truncate_db()?;

        let (s, e) = get_sync_delta_data()?;
        assert_eq!(0, s.len());
        assert_eq!(0, e.len());

        let url = "https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.\
                   com/InterceptedWithJeremyScahill";
        let source = Source::from_url(url)?;
        let id = source.id();
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(vec![source]))?;
        let pd = get_podcast_from_source_id(id)?;

        let eps_num = get_pd_unplayed_episodes(&pd)?.len();
        assert_ne!(eps_num, 0);

        crate::sync::test::init_settings()?;
        let all_podcasts = get_podcasts()?;
        let pd1 = all_podcasts.get(0).unwrap();
        crate::sync::Show::store(pd1, crate::sync::ShowAction::Added)?;

        assert_eq!(1, crate::sync::Show::fetch_all()?.len());

        let (s, e) = get_sync_delta_data()?;
        assert_eq!(1, s.len());
        assert_eq!(0, e.len());

        let mut all_eps = get_episodes()?;
        let ep1 = all_eps.get_mut(0).unwrap();
        crate::sync::Episode::store(ep1.id(), crate::sync::EpisodeAction::Play, Some((0, 30)))?;

        let (s, e) = get_sync_delta_data()?;
        assert_eq!(1, s.len());
        assert_eq!(1, e.len());
        Ok(())
    }
}
