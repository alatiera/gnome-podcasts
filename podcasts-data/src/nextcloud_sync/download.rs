// nextcloud_sync/download.rs
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

use crate::dbqueries;
use crate::feed_manager::FEED_MANAGER;
use crate::models::Episode;
use crate::models::EpisodeModel;
use crate::models::Source;
use crate::nextcloud_sync::data::*;

use anyhow::{Context, Result};

fn update_episodes(data: &EpisodeGet, ignore_missing_episodes: bool) -> Result<(), SyncError> {
    let mut ep_urls: Vec<&str> = data.actions.iter().map(|e| e.episode.as_str()).collect();
    ep_urls.dedup();
    let mut ep_guids: Vec<&str> = data
        .actions
        .iter()
        .filter_map(|e| e.guid.as_deref())
        .collect();
    ep_guids.dedup();
    let local_ep_actions = crate::sync::Episode::fetch_all().context("failed to fetch all")?;
    let mut episodes: Vec<Episode> = dbqueries::get_episodes_by_urls_or_guids(ep_urls, ep_guids)
        .context("failed to get episodes by guids")?;
    let update_error = data.actions.iter().find_map(|ea| {
        //
        let none_updated = episodes
            .iter_mut()
            .filter(|e| {
                // there can be multiple episodes with the same guid,
                // example: when following the paid&free feeds of a podcast and both publish free episodes
                ea.guid.as_deref() == e.guid()
                    || (e.uri().is_some() && ea.episode == e.uri().unwrap())
            })
            .map(|ep| {
                // ignore episode play action if it happened after a local play action and is not a finish
                if let Some(la) = local_ep_actions.iter().find(|la| ep.id() == la.ep_id) {
                    if la.timestamp > ea.timestamp.timestamp()
                        && ea.action == Action::Play
                        && !ea.finished_play()
                    {
                        return;
                    }
                }
                update_episode(ep, ea);
            })
            .count()
            == 0;

        if none_updated {
            error!(
                "Sync: Episode not found locally, faild to update it. ACTION {:#?}",
                ea
            );
            if !ignore_missing_episodes {
                Some(SyncError::DownloadedUpdateForEpisodeNotInDb)
            } else {
                None
            }
        } else {
            None
        }
    });
    if let Some(e) = update_error {
        return Err(e);
    }

    dbqueries::update_episodes(episodes).context("failed to update episodes")?;
    Ok(())
}

// Updates a local episode `ep` from the cloud state `ea`
fn update_episode(ep: &mut Episode, ea: &EpisodeAction) {
    match ea.action {
        Action::Download => (),
        Action::Delete => (),
        Action::Play => {
            ep.set_play_position_no_save(ea.position);
            // Make sure ep is marked as played in the local db.
            if ea.finished_play() {
                ep.set_played(Some(ea.timestamp.naive_utc()));
            }
        }
        Action::New => (), // reset state?
        Action::Flattr => (),
    }
}

async fn fetch_subscription_actions(
    login: &Login,
    last_sync: &Option<i64>,
) -> Result<SubscriptionGet, SyncError> {
    let mut url = login
        .server
        .join("/index.php/apps/gpoddersync/subscriptions")?;
    if let Some(last_sync) = last_sync {
        url.set_query(Some(format!("since={}", last_sync).as_str()));
    }
    debug!("sync: downloading URL {}", url);
    let resp = client_builder()
        .build()?
        .get(url)
        .basic_auth(login.user.clone(), Some(login.password.clone()))
        .send()
        .await?;

    debug!("sync: received response");

    let subs = resp.json::<SubscriptionGet>().await?;
    Ok(subs)
}
async fn fetch_ep_actions(login: &Login, last_sync: &Option<i64>) -> Result<EpisodeGet, SyncError> {
    let mut url = login
        .server
        .join("/index.php/apps/gpoddersync/episode_action")?;
    if let Some(last_sync) = last_sync {
        url.set_query(Some(format!("since={}", last_sync).as_str()));
    }
    debug!("URL {}", url);
    let resp = client_builder()
        .build()?
        .get(url)
        .basic_auth(login.user.clone(), Some(login.password.clone()))
        .send()
        .await?;

    debug!("fetch_ep_actions: RECEIVED RESP");
    // let text = resp.text().await?;
    // debug!("{}", text);
    // bail!("can't parse")
    match resp.json::<EpisodeGet>().await {
        Ok(ep_actions) => Ok(ep_actions),
        Err(e) => Err(SyncError::RequestError(e)),
    }
}

async fn update_subscriptions(data: &SubscriptionGet) -> Result<(), SyncError> {
    let local_showactions = crate::sync::Show::fetch_all()?;
    // remove
    data.remove.iter().for_each(|uri| {
        // ignore removes if an add happened later locally
        if let Some(sa) = local_showactions.iter().find(|sa| uri == &sa.uri) {
            if sa.timestamp > data.timestamp {
                return;
            }
        }
        if let Err(e) = dbqueries::remove_feed_by_uri(uri) {
            debug!("failed to unsubscribe from {uri}: {e}");
        }
    });
    // add
    let sources: Vec<Source> = data
        .add
        .iter()
        .filter_map(|uri| {
            // ignore remote subs if they were unsubbed locally at a later timestamp
            if let Some(sa) = local_showactions.iter().find(|sa| uri == &sa.uri) {
                if sa.timestamp > data.timestamp {
                    return None;
                }
            }
            Source::from_url(uri).ok()
        })
        .collect();
    // fetch the newly subscribed feeds
    FEED_MANAGER.refresh(sources).await;
    Ok(())
}

/// Download changes from the NextCloud Server and applies them to the local database.
/// Returns the downloaded Subscription and Episode changes.
pub(crate) async fn download_changes(
    login: &Login,
    last_sync: Option<i64>,
    ignore_missing_episodes: bool,
) -> Result<(SubscriptionGet, EpisodeGet), SyncError> {
    // fetch new feeds and their episodes first
    // to make sure we have all the episodes for `update_episodes`
    let sub_actions = fetch_subscription_actions(login, &last_sync).await?;
    debug!("SUBS: {:#?}", sub_actions);
    update_subscriptions(&sub_actions).await?;

    let ep_actions = fetch_ep_actions(login, &last_sync).await?;
    debug!("EPAs: {:#?}", ep_actions);
    update_episodes(&ep_actions, ignore_missing_episodes)?;
    Ok((sub_actions, ep_actions))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::dbqueries;
    use crate::nextcloud_sync::test::prepare;
    use crate::pipeline::pipeline;
    use anyhow::Result;
    use http_test_server::http::Status;
    use http_test_server::TestServer;

    #[test]
    fn test_download_changes() -> Result<()> {
        let rt = prepare()?;
        let server = mock_nextcloud_server()?;
        let address = format!("http://127.0.0.1:{}", server.port());

        let login = Login {
            server: parse_url_without_scheme(&address)?,
            user: "test_user".to_string(),
            password: "test_password".to_string(),
        };

        assert_eq!(0, dbqueries::get_podcasts()?.len());
        assert_eq!(0, dbqueries::get_episodes()?.len());

        rt.block_on(download_changes(&login, None, false))?;
        assert_eq!(1, dbqueries::get_podcasts()?.len());
        assert_ne!(0, dbqueries::get_episodes()?.len());
        let all_podcasts = dbqueries::get_podcasts()?;
        let pd1 = all_podcasts.get(0).unwrap();
        let ep = dbqueries::get_episode(None, "Episode 89 - FD Signifier", pd1.id())?;
        assert_eq!(17, ep.play_position());
        assert_eq!(
            Some("gid://art19-episode-locator/V0/t6uIB-2DTTR8nwxR36XLHQMi9ajPfpsTPKBSDWBWsyE"),
            ep.guid()
        );
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

        Ok(server)
    }

    #[test]
    fn test_ignore_sub_remove_before_local_add() -> Result<()> {
        let rt = prepare()?;
        let get = SubscriptionGet {
            add: vec![],
            remove: vec!["https://rss.art19.com/the-deprogram".to_string()],
            timestamp: 0,
        };

        let url = "https://rss.art19.com/the-deprogram";
        let source = Source::from_url(url)?;
        rt.block_on(pipeline(vec![source]))?;

        let all_podcasts = dbqueries::get_podcasts()?;
        let pd1 = all_podcasts.get(0).unwrap();

        crate::sync::test::init_settings()?;
        crate::sync::Show::store(pd1, crate::sync::ShowAction::Added)?;
        assert_eq!(1, crate::sync::Show::fetch_all()?.len());

        rt.block_on(update_subscriptions(&get))?;

        // still subscribed, the remote action is ignored, because it happened before the local add
        let all_podcasts = dbqueries::get_podcasts()?;
        assert!(all_podcasts.get(0).is_some());

        let get = SubscriptionGet {
            add: vec![],
            remove: vec!["https://rss.art19.com/the-deprogram".to_string()],
            timestamp: i64::MAX,
        };

        rt.block_on(update_subscriptions(&get))?;

        // no longer subscribed, the remote action was applied, because it happened after the local one
        let all_podcasts = dbqueries::get_podcasts()?;
        assert!(all_podcasts.get(0).is_none());
        Ok(())
    }

    #[test]
    fn test_ignore_sub_add_before_local_remove() -> Result<()> {
        let rt = prepare()?;
        let get = SubscriptionGet {
            add: vec!["https://rss.art19.com/the-deprogram".to_string()],
            remove: vec![],
            timestamp: 0,
        };

        let all_podcasts = dbqueries::get_podcasts()?;
        assert!(all_podcasts.get(0).is_none());

        crate::sync::test::init_settings()?;
        crate::sync::Show::store_by_uri(
            "https://rss.art19.com/the-deprogram".to_string(),
            crate::sync::ShowAction::Removed,
        )?;
        assert_eq!(1, crate::sync::Show::fetch_all()?.len());

        // remote add is ignored, because there is a local remove action
        rt.block_on(update_subscriptions(&get))?;

        let all_podcasts = dbqueries::get_podcasts()?;
        assert!(all_podcasts.get(0).is_none());

        let get = SubscriptionGet {
            add: vec!["https://rss.art19.com/the-deprogram".to_string()],
            remove: vec![],
            timestamp: i64::MAX,
        };

        rt.block_on(update_subscriptions(&get))?;

        // remote add is applied, because it happened after the local remove action
        let all_podcasts = dbqueries::get_podcasts()?;
        assert!(all_podcasts.get(0).is_some());
        Ok(())
    }
}
