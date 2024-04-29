// nextcloud_sync/mod.rs
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

mod data;
mod download;
mod login;
mod upload;

use crate::nextcloud_sync::data::*;
pub use crate::nextcloud_sync::data::{SyncError, SyncResult};
use crate::nextcloud_sync::download::download_changes;
pub use crate::nextcloud_sync::login::*;
use crate::nextcloud_sync::upload::{make_delta_post, make_initial_post, upload_changes};

use anyhow::{Context, Result};

/// Downloads all updates from the nextcloud server and applies them to the DB.
/// Then it uploads all outstanding local changes to the server.
/// If this is the first sync, it will try to generate an inital changeset from the DB.
/// Once the change upload is done all deltas from the *_sync tables will be deleted and
/// The current date from the start of this fn call will be stored as `last_sync` date.
///
/// if `ignore_missing_episodes` is true it will not cancel
/// when a local episode for a remote episode_action can not be found.
/// The sync will be considered successful even with unapplied episode_actions.
/// This should be set to true for full syncs or inital syncs and false for quick syncs that only push out an update.
/// This option is provided, because ignoring an episode update is less bad than never completing a sync,
/// and having the Error provides an option to fall back and update all feeds form a quick sync.
///
/// If sync is not configured, it will return Ok(false)

pub async fn sync(ignore_missing_episodes: bool) -> Result<SyncResult, SyncError> {
    if let Ok((settings, password)) = crate::sync::Settings::fetch().await {
        if !settings.active {
            // sync is turned off, skip
            return Ok(SyncResult::Skipped);
        }

        let login = Login {
            server: parse_url_without_scheme(&settings.server)?,
            user: settings.user.to_owned(),
            password,
        };

        sync_for_login(login, settings, ignore_missing_episodes).await
    } else {
        // sync is not configured, skip
        Ok(SyncResult::Skipped)
    }
}

/// Refer to sync() doc.
/// This was split so it can be tested without calling oo7 password restore.
async fn sync_for_login(
    login: Login,
    settings: crate::sync::Settings,
    ignore_missing_episodes: bool,
) -> Result<SyncResult, SyncError> {
    let now = chrono::Utc::now();
    let (dl_sub_actions, dl_ep_actions) =
        download_changes(&login, settings.last_sync, ignore_missing_episodes).await?;

    let (sub_actions, ep_actions) = if settings.did_first_sync() {
        make_delta_post(&now, &dl_ep_actions)?
    } else {
        let (mut sub_actions, ep_actions) = make_initial_post(&now)?;
        sub_actions.remove_already_on_server(&dl_sub_actions);
        // only send actions that aren't already on the server.
        let ep_actions = ep_actions
            .into_iter()
            .filter(|e| !e.already_on_server(&dl_ep_actions))
            .collect();

        (sub_actions, ep_actions)
    };

    upload_changes(&login, sub_actions, ep_actions).await?;
    crate::sync::delete_deltas(now).context("failed to delete deltas")?;
    Ok(SyncResult::Done {
        episode_updates_downloaded: dl_ep_actions.actions.len(),
        subscription_updates_downloaded: dl_sub_actions.add.len() + dl_sub_actions.remove.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::*;
    use crate::nextcloud_sync::test::prepare;
    use anyhow::Result;
    use http_test_server::http::{Method, Status};
    use http_test_server::TestServer;

    #[test]
    fn test_skip() -> Result<()> {
        let rt = prepare()?;
        let result = rt.block_on(sync(false))?;
        assert_eq!(SyncResult::Skipped, result);
        Ok(())
    }
    #[test]
    fn test_pass() -> Result<()> {
        let rt = prepare()?;
        let server = mock_nextcloud_server()?;
        let address = format!("http://127.0.0.1:{}", server.port());
        crate::sync::Settings::store_entry(&address, "user")?;
        let login = Login {
            server: parse_url_without_scheme(&address)?,
            user: "test_user".to_string(),
            password: "test_password".to_string(),
        };
        let settings = crate::sync::Settings::fetch_entry()?;
        rt.block_on(sync_for_login(login, settings, false))?;
        Ok(())
    }

    #[test]
    fn test_error() -> Result<()> {
        let rt = prepare()?;
        // no server started
        let address = format!("http://127.0.0.1:{}", 80);
        crate::sync::Settings::store_entry(&address, "user")?;
        let login = Login {
            server: parse_url_without_scheme(&address)?,
            user: "test_user".to_string(),
            password: "test_password".to_string(),
        };
        let settings = crate::sync::Settings::fetch_entry()?;
        let result = rt.block_on(sync_for_login(login, settings, false));
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_error_missing_episodes() -> Result<()> {
        let rt = prepare()?;
        let server = mock_nextcloud_server_missing()?;
        let address = format!("http://127.0.0.1:{}", server.port());
        crate::sync::Settings::store_entry(&address, "user")?;
        let login = Login {
            server: parse_url_without_scheme(&address)?,
            user: "test_user".to_string(),
            password: "test_password".to_string(),
        };
        let settings = crate::sync::Settings::fetch_entry()?;
        match rt.block_on(sync_for_login(login, settings, false)) {
            Err(SyncError::DownloadedUpdateForEpisodeNotInDb) => (),
            _ => panic!(),
        }
        Ok(())
    }

    #[test]
    fn test_skip_error_missing_episodes() -> Result<()> {
        let rt = prepare()?;
        let server = mock_nextcloud_server_missing()?;
        let address = format!("http://127.0.0.1:{}", server.port());
        crate::sync::Settings::store_entry(&address, "user")?;
        let login = Login {
            server: parse_url_without_scheme(&address)?,
            user: "test_user".to_string(),
            password: "test_password".to_string(),
        };
        let settings = crate::sync::Settings::fetch_entry()?;
        // check that we don't get DownloadedUpdateForEpisodeNotInDb errors when passing true here
        if let Err(SyncError::DownloadedUpdateForEpisodeNotInDb) =
            rt.block_on(sync_for_login(login, settings, true))
        {
            panic!();
        }
        Ok(())
    }

    fn mock_nextcloud_server() -> Result<TestServer> {
        let server = TestServer::new()?;

        server
            .create_resource("/index.php/apps/gpoddersync/subscriptions")
            .status(Status::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(
                r#"{"add": ["https://rss.art19.com/the-deprogram"], "remove": [], "timestamp": 0}"#,
            );

        server
            .create_resource("/index.php/apps/gpoddersync/episode_action")
            .status(Status::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(crate::nextcloud_sync::data::test::TEST_ACTIONS);

        server
            .create_resource("/index.php/apps/gpoddersync/subscription_change/create")
            .status(Status::OK)
            .method(Method::POST)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(r#"{"timestamp":1694997647}"#);

        server
            .create_resource("/index.php/apps/gpoddersync/episode_action/create")
            .status(Status::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(r#"{"timestamp":1694997647}"#);

        Ok(server)
    }

    fn mock_nextcloud_server_missing() -> Result<TestServer> {
        let server = TestServer::new()?;

        server
            .create_resource("/index.php/apps/gpoddersync/subscriptions")
            .status(Status::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(
                r#"{"add": ["https://rss.art19.com/the-deprogram", ""], "remove": [], "timestamp": 0}"#,
            );

        server
            .create_resource("/index.php/apps/gpoddersync/episode_action")
            .status(Status::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(crate::nextcloud_sync::data::test::TEST_ACTIONS_WITH_MISSING_SUB);

        server
            .create_resource("/index.php/apps/gpoddersync/subscription_change/create")
            .status(Status::OK)
            .method(Method::POST)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(r#"{"timestamp":1694997647}"#);

        server
            .create_resource("/index.php/apps/gpoddersync/episode_action/create")
            .status(Status::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .body(r#"{"timestamp":1694997647}"#);

        Ok(server)
    }
}
