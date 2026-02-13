// sync/mod.rs
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

// Sync datatypes to store updates that still have to be sent out
// This is mostly glue code for the DB, use store(), fetch(), delete() methods to interact

use crate::database::connection;
use crate::errors::DataError;
use diesel::prelude::*;

mod episode;
mod settings;
mod show;

pub use crate::models::sync::episode::*;
pub use crate::models::sync::settings::*;
pub use crate::models::sync::show::*;

/// Clears all Show and Episode sync delta data and sets the last_sync date in `settings_sync` to `now`.
pub(crate) fn delete_deltas(now: chrono::DateTime<chrono::Utc>) -> Result<(), DataError> {
    use crate::schema::episodes_sync::dsl::episodes_sync;
    use crate::schema::settings_sync::dsl::*;
    use crate::schema::shows_sync::dsl::shows_sync;
    let db = connection();
    let mut con = db.get()?;

    con.transaction::<(), DataError, _>(|conn| {
        diesel::delete(
            episodes_sync.filter(crate::schema::episodes_sync::dsl::timestamp.le(now.timestamp())),
        )
        .execute(conn)?;
        diesel::delete(
            shows_sync.filter(crate::schema::shows_sync::dsl::timestamp.le(now.timestamp())),
        )
        .execute(conn)?;
        diesel::update(settings_sync)
            .set(last_sync.eq(now.timestamp()))
            .execute(conn)?;
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::database::*;
    use crate::dbqueries;
    use crate::models::Source;
    use crate::pipeline::pipeline;
    use crate::test_feeds::*;
    use anyhow::Result;

    pub(crate) fn init_settings() -> Result<()> {
        let now = chrono::Utc::now();
        crate::models::sync::Settings::store_entry("127.0.0.1", "test_user")?;
        delete_deltas(now)?;
        Ok(())
    }

    #[test]
    fn test_delete_deltas() -> Result<()> {
        let _tempfile = reset_db()?;

        // starts empty
        assert!(Settings::fetch_entry().is_err());
        assert_eq!(0, Show::fetch_all()?.len());
        assert_eq!(0, Episode::fetch_all()?.len());
        delete_deltas(chrono::Utc::now())?;
        // still empty after delete
        assert!(Settings::fetch_entry().is_err());
        assert_eq!(0, Show::fetch_all()?.len());
        assert_eq!(0, Episode::fetch_all()?.len());

        // insert show delta
        init_settings()?;
        assert!(Settings::fetch_entry()?.did_first_sync());
        Show::store_by_uri("test".to_string(), ShowAction::Added)?;
        assert_eq!(1, Show::fetch_all()?.len());

        let rt = tokio::runtime::Runtime::new()?;
        let server = mock_feed_server()?;
        let feed_url = mock_feed_url(&server, MOCK_FEED_DEPROGRAM);
        let source = Source::from_url(&feed_url)?;
        rt.block_on(pipeline(vec![source]))?;

        // insert ep delta
        let all_eps = dbqueries::get_episodes()?;
        let ep1 = all_eps.get(0).unwrap();
        Episode::store(ep1.id(), EpisodeAction::Downloaded, None)?;
        assert_eq!(1, Episode::fetch_all()?.len());

        // delete removes deltas, keeps settings
        let now = chrono::Utc::now();
        delete_deltas(now)?;
        assert!(Settings::fetch_entry().is_ok());
        assert_eq!(Some(now.timestamp()), Settings::fetch_entry()?.last_sync);
        assert_eq!(0, Show::fetch_all()?.len());
        assert_eq!(0, Episode::fetch_all()?.len());

        // deltas that are newer than the time passed to delete_deltas won't be deleted
        std::thread::sleep(std::time::Duration::new(2, 0));
        Show::store_by_uri("test".to_string(), ShowAction::Added)?;
        assert_eq!(1, Show::fetch_all()?.len());
        Episode::store(ep1.id(), EpisodeAction::Downloaded, None)?;
        assert_eq!(1, Episode::fetch_all()?.len());
        delete_deltas(now)?;
        assert!(Settings::fetch_entry().is_ok());
        assert_eq!(Some(now.timestamp()), Settings::fetch_entry()?.last_sync);
        assert!(Settings::fetch_entry()?.last_sync_local().is_some());
        assert_eq!(1, Show::fetch_all()?.len());
        assert_eq!(1, Episode::fetch_all()?.len());

        Ok(())
    }
}
