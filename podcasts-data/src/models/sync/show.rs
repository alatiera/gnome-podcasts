// sync/show.rs
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

use crate::database::connection;
use crate::errors::DataError;
use crate::models::sync::settings::Settings;
use crate::schema::shows_sync;

#[derive(Insertable, Queryable, Identifiable, AsChangeset, PartialEq)]
#[diesel(table_name = shows_sync)]
#[diesel(treat_none_as_null = true)]
#[diesel(primary_key(uri))]
#[derive(Debug, Clone)]
// required for batch upsert on sqlite
// https://github.com/diesel-rs/diesel/discussions/4545#discussioncomment-12568645
#[diesel(treat_none_as_default_value = false)]
/// Stores Show subscription updates that sill have to be sent to the server.
pub struct Show {
    /// Uri/url of the podcast.
    pub(crate) uri: String,
    /// new Uri/url of the podcast used for update action
    pub(crate) new_uri: Option<String>,
    action: String,
    /// When this action happened locally. UTC timestamp.
    pub(crate) timestamp: i64,
}

#[derive(Debug, Clone, PartialEq)]
/// Describes what was done to a Show on this device.
pub enum ShowAction {
    /// A Show was subscribed to.
    Added,
    /// A Show was unsubscribed.
    Removed,
    /// A feed moved by permanent redirect. It is now at new_uri.
    Moved(String),
}

impl Show {
    /// Store a Show Action to be synced between devices.
    /// Will not store anything if sync isn't configured.
    pub fn store(show: &crate::models::show::Show, action: ShowAction) -> Result<(), DataError> {
        let source = crate::dbqueries::get_source_from_id(show.source_id())?;
        Self::store_by_uri(source.uri().to_owned(), action)
    }
    /// Store a Show Action to be synced between devices.
    /// Will not store anything if sync isn't configured.
    /// Prefer using ::store when the Show is available during Remove.
    pub fn store_by_uri(uri: String, action: ShowAction) -> Result<(), DataError> {
        if !Settings::fetch_entry()
            .ok()
            .map(|s| s.did_first_sync())
            .unwrap_or(false)
        {
            debug!("sync: NOT STORING CHANGE, sync is not configured");
            return Ok(());
        }
        let now = chrono::Utc::now();
        let new_uri = if let ShowAction::Moved(u) = &action {
            Some(u.clone())
        } else {
            None
        };

        let s = Show {
            uri,
            new_uri,
            action: match action {
                ShowAction::Added => "A".to_owned(),
                ShowAction::Removed => "R".to_owned(),
                ShowAction::Moved(_) => "M".to_owned(),
            },
            timestamp: now.timestamp(),
        };
        debug!("UPSERTING");
        s.upsert()?;
        debug!("DONE UPSERTING");
        Ok(())
    }

    /// Store multiple Show Added Actions to be synced between devices.
    /// Will not store anything if sync isn't configured.
    pub fn store_multiple_subscriptions(uris: &[String]) -> Result<(), DataError> {
        if !Settings::fetch_entry()
            .ok()
            .map(|s| s.did_first_sync())
            .unwrap_or(false)
        {
            debug!("sync: NOT STORING CHANGE, sync is not configured");
            return Ok(());
        }
        let now = chrono::Utc::now();

        let shows: Vec<_> = uris
            .iter()
            .map(|u| Show {
                uri: u.to_string(),
                new_uri: None,
                action: "A".to_owned(),
                timestamp: now.timestamp(),
            })
            .collect();

        use crate::schema::shows_sync::dsl::*;
        use diesel::upsert::excluded;
        let db = connection();
        let mut con = db.get()?;

        info!("Upserting multiple sync subscriptions {:?}", uris);
        diesel::insert_into(shows_sync)
            .values(&shows)
            .on_conflict(uri)
            .do_update()
            .set((
                uri.eq(excluded(uri)),
                action.eq(excluded(action)),
                timestamp.eq(excluded(timestamp)),
            ))
            .execute(&mut con)
            .map_err(From::from)
            .map(|_| ())
    }

    /// May return None if invalid data is in the database.
    pub(crate) fn action(&self) -> Option<ShowAction> {
        match self.action.as_str() {
            "A" => Some(ShowAction::Added),
            "R" => Some(ShowAction::Removed),
            "M" => self
                .new_uri
                .as_ref()
                .map(|uri| ShowAction::Moved(uri.clone())),
            _ => None,
        }
    }

    /// Returns all show deltas that still need to be synced with the remote server.
    pub(crate) fn fetch_all() -> Result<Vec<Self>, DataError> {
        use crate::schema::shows_sync::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        let all = shows_sync.load::<Self>(&mut con)?;
        Ok(all)
    }

    fn upsert(&self) -> Result<(), DataError> {
        use crate::schema::shows_sync::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        info!("Inserting {:?}", self.uri);
        diesel::insert_into(shows_sync)
            .values(self)
            .on_conflict(uri)
            .do_update()
            .set(self)
            .execute(&mut con)
            .map_err(From::from)
            .map(|_| ())
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
        let _tempfile = reset_db()?;
        crate::models::sync::test::init_settings()?;
        let rt = tokio::runtime::Runtime::new()?;
        let url = "https://rss.art19.com/the-deprogram";
        let url2 = "test";
        let source = Source::from_url(url)?;
        rt.block_on(pipeline(vec![source]))?;

        // insert ep delta
        let all_podcasts = dbqueries::get_podcasts()?;
        let pd1 = all_podcasts.get(0).unwrap();
        Show::store(pd1, ShowAction::Added)?;
        assert_eq!(1, Show::fetch_all()?.len());
        Show::store_by_uri(url2.to_string(), ShowAction::Added)?;
        assert_eq!(2, Show::fetch_all()?.len());

        // inserting on the same show, updates
        Show::store(pd1, ShowAction::Removed)?;
        Show::store(pd1, ShowAction::Moved(url2.to_string()))?;
        Show::store_by_uri(url2.to_string(), ShowAction::Removed)?;
        Show::store_by_uri(url2.to_string(), ShowAction::Moved(url.to_string()))?;
        assert_eq!(2, Show::fetch_all()?.len());
        let all_deltas = Show::fetch_all()?;

        // check the actions
        assert_eq!(
            Some(ShowAction::Moved(url2.to_string())),
            all_deltas.get(0).unwrap().action()
        );
        assert_eq!(
            Some(ShowAction::Moved(url.to_string())),
            all_deltas.get(1).unwrap().action()
        );

        Show::store(pd1, ShowAction::Removed)?;
        let all_deltas = Show::fetch_all()?;
        assert_eq!(
            Some(ShowAction::Removed),
            all_deltas.get(0).unwrap().action()
        );

        Show::store(pd1, ShowAction::Added)?;
        let all_deltas = Show::fetch_all()?;
        assert_eq!(Some(ShowAction::Added), all_deltas.get(0).unwrap().action());

        Ok(())
    }
}
