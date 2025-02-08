// sync/episode.rs
//
// Copyright 2023-2024 nee <nee-git@patchouli.garden>
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

use diesel::prelude::*;

use crate::EpisodeId;
use crate::database::connection;
use crate::errors::DataError;
use crate::models::sync::settings::Settings;
use crate::schema::episodes_sync;

#[derive(Insertable, Queryable, Identifiable, AsChangeset, PartialEq)]
#[diesel(table_name = episodes_sync)]
#[diesel(treat_none_as_null = true)]
#[diesel(primary_key(ep_id, action))]
#[derive(Debug, Clone)]
/// Stores Episode updates that sill have to be sent to the server.
pub struct Episode {
    /// Local db id of the episode.
    pub(crate) ep_id: EpisodeId,
    action: String,
    /// When this action happened locally. UTC timestamp.
    pub(crate) timestamp: i64,
    /// Where playback was started from. Only used for Play Actions.
    pub(crate) start: Option<i32>,
    /// Where playback was stopped. Only used for Play Actions.
    pub(crate) position: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Describes what was done to an episode on this device.
pub enum EpisodeAction {
    /// An episode was played to `position`.
    Play,
    /// An episode was finished.
    Finished,
    /// An episode was downloaded, the file is on this device.
    Downloaded,
    /// An episode file on this device was deleted.
    Deleted,
}

impl Episode {
    /// Store an Episode Action to be synced between devices.
    /// Will not store anything if sync isn't configured.
    pub fn store(
        ep_id: EpisodeId,
        action: EpisodeAction,
        times: Option<(i32, i32)>,
    ) -> Result<(), DataError> {
        if !Settings::fetch_entry()
            .ok()
            .map(|s| s.did_first_sync())
            .unwrap_or(false)
        {
            debug!("sync: NOT STORING CHANGE, sync is not configured");
            return Ok(());
        }
        let now = chrono::Utc::now();
        let ep = Episode {
            ep_id,
            action: match action {
                EpisodeAction::Play => "P".to_owned(),
                EpisodeAction::Finished => "F".to_owned(),
                EpisodeAction::Downloaded => "D".to_owned(),
                EpisodeAction::Deleted => "R".to_owned(),
            },
            timestamp: now.timestamp(),
            start: times.map(|(s, _)| s),
            position: times.map(|(_, p)| p),
        };

        ep.upsert()?;
        Ok(())
    }
    /// May return None if invalid data is in the database.
    pub(crate) fn action(&self) -> Option<EpisodeAction> {
        match self.action.as_str() {
            "P" => Some(EpisodeAction::Play),
            "F" => Some(EpisodeAction::Finished),
            "D" => Some(EpisodeAction::Downloaded),
            "R" => Some(EpisodeAction::Deleted),
            _ => None,
        }
    }

    /// Returns all show deltas that still need to be synced with the remote server.
    pub(crate) fn fetch_all() -> Result<Vec<Self>, DataError> {
        use crate::schema::episodes_sync::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        let all = episodes_sync.load::<Self>(&mut con)?;
        Ok(all)
    }

    fn upsert(&self) -> Result<(), DataError> {
        use crate::schema::episodes_sync::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        debug!("UPSERTING EP");

        // on conflict: only update position,
        // keep start, because the played timeframe still starts at the first position
        // other fields are part of the id and need no update
        // TODO FIXME MAKE SURE `start` IS BEFORE `position`
        info!("Inserting {:?} {:?}", self.ep_id, self.action);
        let result = diesel::insert_into(episodes_sync)
            .values(self)
            .on_conflict((ep_id, action))
            .do_update()
            .set((position.eq(self.position), timestamp.eq(self.timestamp)))
            .execute(&mut con)
            .map_err(From::from)
            .map(|_| ());
        debug!("UPSERTING EP DONE");
        result
    }

    /// Ignore if this is a Play that happened before the timestamp (of another play action).
    /// Finished plays are not ignored, because we store them as EpisodeAction::Finished.
    pub fn should_be_ignored(&self, timestamp: i64) -> bool {
        timestamp > self.timestamp && self.action().eq(&Some(EpisodeAction::Play))
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::database::*;
    use crate::dbqueries;
    use crate::models::Source;
    use crate::pipeline::pipeline;
    use anyhow::Result;

    #[test]
    fn test_episode_deltas() -> Result<()> {
        truncate_db()?;
        crate::models::sync::test::init_settings()?;
        let rt = tokio::runtime::Runtime::new()?;
        let url = "https://rss.art19.com/the-deprogram";
        let source = Source::from_url(url)?;
        rt.block_on(pipeline(vec![source]))?;

        // insert ep delta
        let all_eps = dbqueries::get_episodes()?;
        let ep1 = all_eps.get(0).unwrap();
        Episode::store(ep1.id(), EpisodeAction::Play, Some((0, 15)))?;
        Episode::store(ep1.id(), EpisodeAction::Finished, None)?;
        Episode::store(ep1.id(), EpisodeAction::Downloaded, None)?;
        Episode::store(ep1.id(), EpisodeAction::Deleted, None)?;
        assert_eq!(4, Episode::fetch_all()?.len());
        Episode::store(ep1.id(), EpisodeAction::Play, Some((0, 30)))?;
        assert_eq!(4, Episode::fetch_all()?.len());
        let all_deltas = Episode::fetch_all()?;
        assert_eq!(
            Some(EpisodeAction::Play),
            all_deltas.get(0).unwrap().action()
        );
        assert_eq!(
            Some(EpisodeAction::Finished),
            all_deltas.get(1).unwrap().action()
        );
        assert_eq!(
            Some(EpisodeAction::Downloaded),
            all_deltas.get(2).unwrap().action()
        );
        assert_eq!(
            Some(EpisodeAction::Deleted),
            all_deltas.get(3).unwrap().action()
        );

        let remote_action_timestamp_old = chrono::Utc::now().timestamp() - 500;
        let remote_action_timestamp_new = chrono::Utc::now().timestamp() + 500;

        let delta1 = all_deltas.get(0).unwrap();
        assert_eq!(Some(30), delta1.position);

        let ignore1 = delta1.should_be_ignored(remote_action_timestamp_old);
        let ignore2 = delta1.should_be_ignored(remote_action_timestamp_new);
        assert!(!ignore1);
        assert!(ignore2);

        Ok(())
    }
}
