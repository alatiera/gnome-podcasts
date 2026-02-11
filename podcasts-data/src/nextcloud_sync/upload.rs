// nextcloud_sync/upload.rs
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

use crate::EpisodeModel;
use crate::dbqueries;
use crate::nextcloud_sync::data::*;

use anyhow::{Context, Result};
use serde::Serialize;

pub(crate) async fn upload_changes(
    login: &Login,
    sub_actions: SubscriptionPost,
    ep_actions: Vec<EpisodeAction>,
) -> Result<(), SyncError> {
    if !sub_actions.is_empty() {
        post_json(
            login,
            "/index.php/apps/gpoddersync/subscription_change/create",
            &sub_actions,
        )
        .await?;
    }
    if !ep_actions.is_empty() {
        post_json(
            login,
            "/index.php/apps/gpoddersync/episode_action/create",
            &ep_actions,
        )
        .await?;
    }
    Ok(())
}

async fn post_json<T>(login: &Login, path: &str, body: T) -> Result<(), SyncError>
where
    T: Serialize,
{
    let url = login.server.join(path)?;
    debug!("url {:#?}", url);
    debug!("{}", serde_json::to_string_pretty(&body).unwrap());
    let resp = client_builder()
        .build()?
        .post(url)
        .basic_auth(login.user.clone(), Some(login.password.clone()))
        .json(&body)
        .send()
        .await?;

    debug!("received RESP");

    let status = resp.status();
    let text = resp.text().await?;
    debug!("{}", text);
    if status == 200 {
        debug!("SYNC POST_JSON RESP: {}", text);
        return Ok(());
    }
    Err(SyncError::UnexpectedResponse(status))
}

pub(crate) fn make_initial_post(
    now: &chrono::DateTime<chrono::Utc>,
) -> Result<(SubscriptionPost, Vec<EpisodeAction>), SyncError> {
    let shows_with_sources =
        dbqueries::get_podcast_ids_to_uris().context("failed to map ids to uris")?;
    let all_eps = dbqueries::get_episodes().context("failed to get all episodes")?;
    let ep_actions = all_eps
        .into_iter()
        .flat_map(|e| {
            let show_uri = shows_with_sources.get(&e.show_id());
            actions_for_ep(now, &e, show_uri.map(|s| s.as_str()))
        })
        .collect();

    let show_actions = SubscriptionPost {
        add: shows_with_sources.into_values().collect(),
        remove: vec![],
    };

    Ok((show_actions, ep_actions))
}

//  EpisodeGet used to check for events that should be ignored
pub(crate) fn make_delta_post(
    now: &chrono::DateTime<chrono::Utc>,
    downloaded_actions: &EpisodeGet,
) -> Result<(SubscriptionPost, Vec<EpisodeAction>), SyncError> {
    let (show_deltas, ep_deltas) =
        dbqueries::get_sync_delta_data().context("failed to get delta data")?;
    let mut ep_actions: Vec<EpisodeAction> = ep_deltas
        .into_iter()
        .filter_map(|(e, ep, _show, source)| {
            let action = e.action()?;
            let empty_string = "".to_owned();
            let show_uri = source.uri();
            let episode_uri = ep.uri().unwrap_or(&empty_string);
            let guid = ep.guid().map(|s| s.to_owned());
            let position = e.position.unwrap_or(0);
            // TODO FIXME no unwrap
            let timestamp =
                chrono::DateTime::<chrono::Utc>::from_timestamp(e.timestamp, 0).unwrap();

            // Filter out episode updates that have a more recent timestamp on the server
            // than locally.
            //
            // It should be okay to send this,
            // the '?since=' param should make the server not send these to other devices,
            // but this saves bandwidth and seems pointless to send them.
            let ignore = downloaded_actions
                .actions
                .iter()
                .filter(|de| {
                    (!episode_uri.is_empty() && de.episode == episode_uri)
                        || (guid.is_some() && de.guid == guid)
                })
                .map(|d| d.timestamp.timestamp())
                .any(|remote_ep_timestamp| e.should_be_ignored(remote_ep_timestamp));
            if ignore {
                info!("sync: ignore ep delta update, already updated on server");
                return None;
            }

            match action {
                crate::sync::EpisodeAction::Play => Some(EpisodeAction {
                    podcast: show_uri.to_owned(),
                    episode: episode_uri.to_owned(),
                    guid,
                    action: Action::Play,
                    timestamp,
                    started: e.start.unwrap_or(0), // where PLAY started
                    position,                      // where PLAY ended
                    total: ep.duration().unwrap_or(position), // total file duration
                }),
                crate::sync::EpisodeAction::Finished => Some(EpisodeAction {
                    podcast: show_uri.to_owned(),
                    episode: episode_uri.to_owned(),
                    guid,
                    action: Action::Play,
                    timestamp,
                    started: e.start.unwrap_or(0),
                    position: ep.duration().unwrap_or(0),
                    total: ep.duration().unwrap_or(position),
                }),
                crate::sync::EpisodeAction::Downloaded => Some(EpisodeAction {
                    podcast: show_uri.to_owned(),
                    episode: episode_uri.to_owned(),
                    guid,
                    action: Action::Download,
                    timestamp,
                    started: -1,
                    position: -1,
                    total: -1,
                }),
                crate::sync::EpisodeAction::Deleted => Some(EpisodeAction {
                    podcast: show_uri.to_owned(),
                    episode: episode_uri.to_owned(),
                    guid,
                    action: Action::Delete,
                    timestamp,
                    started: -1,
                    position: -1,
                    total: -1,
                }),
            }
        })
        .collect();

    let show_actions = SubscriptionPost {
        add: show_deltas
            .iter()
            .filter_map(|(s, _, _)| {
                if let Some(crate::sync::ShowAction::Added) = s.action() {
                    Some(s.uri.clone())
                } else if let Some(crate::sync::ShowAction::Moved(new_uri)) = s.action() {
                    Some(new_uri)
                } else {
                    None
                }
            })
            .collect(),
        remove: show_deltas
            .iter()
            .filter_map(|(s, _, _)| {
                if let Some(crate::sync::ShowAction::Removed) = s.action() {
                    Some(s.uri.clone())
                } else if let Some(crate::sync::ShowAction::Moved(_)) = s.action() {
                    Some(s.uri.clone())
                } else {
                    None
                }
            })
            .collect(),
    };

    // RESEND ALL EPISODE ACTIONS FOR A MOVED SHOW
    let mut move_eps_action = show_deltas
        .iter()
        .flat_map(|(s, _, show)| {
            if let Some(crate::sync::ShowAction::Moved(show_uri)) = s.action() {
                if let Some(show) = show {
                    if let Ok(eps) = dbqueries::get_pd_episodes(show) {
                        return eps
                            .into_iter()
                            .flat_map(|e| actions_for_ep(now, &e, Some(show_uri.as_str())))
                            .collect();
                    }
                }
            }
            vec![]
        })
        .collect();
    ep_actions.append(&mut move_eps_action);

    Ok((show_actions, ep_actions))
}

fn actions_for_ep(
    now: &chrono::DateTime<chrono::Utc>,
    e: &crate::Episode,
    uri: Option<&str>,
) -> Vec<EpisodeAction> {
    let mut result = vec![];

    // no way to link the episode to a show
    if uri.is_none() && e.guid().is_none() {
        info!("sync: skip sending episode, no uri");
        return vec![];
    }

    let empty_str = "".to_owned();
    let show_uri = uri.unwrap_or(&empty_str);
    let episode_uri = e.uri().unwrap_or(&empty_str);
    let guid = e.guid().map(|s| s.to_owned());
    if e.local_uri().is_some() {
        result.push(EpisodeAction {
            podcast: show_uri.to_owned(),
            episode: episode_uri.to_owned(),
            guid: guid.clone(),
            action: Action::Download,
            timestamp: *now,
            started: -1,  // where PLAY started
            position: -1, // where PLAY ended
            total: -1,    // total file duration
        });
    }

    // mark as played by setting play at the end of duration
    if e.played().is_some() {
        result.push(EpisodeAction {
            podcast: show_uri.to_owned(),
            episode: episode_uri.to_owned(),
            guid: guid.clone(),
            action: Action::Play,
            timestamp: *now,
            started: 0,                                          // where PLAY started
            position: e.duration().unwrap_or(e.play_position()), // where PLAY ended
            total: e.duration().unwrap_or(e.play_position()),    // total file duration
        });
    }

    if e.play_position() != 0 {
        result.push(EpisodeAction {
            podcast: show_uri.to_owned(),
            episode: episode_uri.to_owned(),
            guid,
            action: Action::Play,
            timestamp: *now,
            started: 0,                                       // where PLAY started
            position: e.play_position(),                      // where PLAY ended
            total: e.duration().unwrap_or(e.play_position()), // total file duration
        });
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::database::*;
    use crate::dbqueries;
    use crate::models::Save;
    use crate::models::Source;
    use crate::pipeline::pipeline;
    use anyhow::Result;
    use http_test_server::TestServer;
    use http_test_server::http::{Method, Status};

    fn ep_get() -> EpisodeGet {
        EpisodeGet {
            actions: vec![],
            timestamp: 0,
        }
    }

    #[test]
    fn test_inital_post_empty() -> Result<()> {
        let _tempfile = reset_db()?;
        let now = chrono::Utc::now();
        let (s, e) = make_initial_post(&now)?;
        assert!(s.is_empty());
        assert!(e.is_empty());
        Ok(())
    }
    #[test]
    fn test_inital_post() -> Result<()> {
        let _tempfile = reset_db()?;
        let rt = tokio::runtime::Runtime::new()?;
        let url = "https://rss.art19.com/the-deprogram";
        let source = Source::from_url(url)?;
        rt.block_on(pipeline(vec![source]))?;
        let mut all_eps = dbqueries::get_episodes()?;
        assert_ne!(0, all_eps.len());

        let now = chrono::Utc::now();
        let (s, e) = make_initial_post(&now)?;
        debug!("{:#?} {:#?}", s, e);
        assert!(!s.is_empty());
        assert_eq!(0, e.len()); // no playtimes/downloads on the episodes

        // make sure we have 1 ep update
        let ep1 = all_eps.get_mut(0).unwrap();
        ep1.set_play_position_and_save(ep1.duration().unwrap())?;

        let (s, e) = make_initial_post(&now)?;
        debug!("{:#?} {:#?}", s, e);
        assert!(!s.is_empty());
        assert_eq!(1, e.len());

        // add a DOWNLOAD episode update
        let mut ep1w = dbqueries::get_episode_widget_from_id(ep1.id())?;
        ep1w.set_local_uri(Some("tmp"));
        ep1w.save()?;

        let (s, e) = make_initial_post(&now)?;
        debug!("{:#?} {:#?}", s, e);
        assert!(!s.is_empty());
        assert_eq!(2, e.len());
        Ok(())
    }
    #[test]
    fn test_delta_post() -> Result<()> {
        let _tempfile = reset_db()?;

        let now = chrono::Utc::now();
        let (s, e) = make_delta_post(&now, &ep_get())?;
        assert!(s.is_empty());
        assert!(e.is_empty());

        let rt = tokio::runtime::Runtime::new()?;
        let url = "https://rss.art19.com/the-deprogram";
        let source = Source::from_url(url)?;
        rt.block_on(pipeline(vec![source]))?;
        let mut all_eps = dbqueries::get_episodes()?;
        assert_ne!(0, all_eps.len());

        // make sure settings are there, otherwise ::store will be skipped
        crate::sync::test::init_settings()?;

        let all_podcasts = dbqueries::get_podcasts()?;
        let pd1 = all_podcasts.get(0).unwrap();
        crate::sync::Show::store(pd1, crate::sync::ShowAction::Added)?;
        assert_eq!(1, crate::sync::Show::fetch_all()?.len());
        // make sure we have 1 ep update
        let ep1 = all_eps.get_mut(0).unwrap();
        crate::sync::Episode::store(ep1.id(), crate::sync::EpisodeAction::Play, Some((0, 15)))?;
        assert_eq!(1, crate::sync::Episode::fetch_all()?.len());
        let (s, e) = make_delta_post(&now, &ep_get())?;
        assert_eq!(1, s.add.len());
        assert_eq!(1, e.len());

        // This should be an upate, not an insert
        crate::sync::Episode::store(ep1.id(), crate::sync::EpisodeAction::Play, Some((0, 30)))?;
        assert_eq!(1, crate::sync::Episode::fetch_all()?.len());

        let (s, e) = make_delta_post(&now, &ep_get())?;
        assert_eq!(1, s.add.len());
        assert_eq!(1, e.len());

        Ok(())
    }
    #[test]
    fn test_show_moved_post() -> Result<()> {
        let _tempfile = reset_db()?;

        let now = chrono::Utc::now();
        let (s, e) = make_delta_post(&now, &ep_get())?;
        assert!(s.is_empty());
        assert!(e.is_empty());

        let rt = tokio::runtime::Runtime::new()?;
        let url1 = "https://web.archive.org/web/20220110083840if_/\
                   https://rss.art19.com/the-deprogram";
        let url2 = "https://web.archive.org/web/20220120083840if_/\
                   https://rss.art19.com/the-deprogram";

        let source = Source::from_url(url1)?;
        rt.block_on(pipeline(vec![source]))?;
        let mut all_eps = dbqueries::get_episodes()?;
        assert_ne!(0, all_eps.len());

        // make sure settings are there, otherwise ::store will be skipped
        crate::sync::test::init_settings()?;

        crate::sync::Show::store_by_uri(
            url1.to_string(),
            crate::sync::ShowAction::Moved(url2.to_string()),
        )?;

        let (s, e) = make_delta_post(&now, &ep_get())?;
        assert_eq!(1, s.add.len());
        assert_eq!(1, s.remove.len());
        assert_eq!(0, e.len());

        assert_eq!(url1, s.remove.get(0).unwrap());
        assert_eq!(url2, s.add.get(0).unwrap());

        let ep1 = all_eps.get_mut(0).unwrap();
        ep1.set_play_position_and_save(15)?;

        let (s, e) = make_delta_post(&now, &ep_get())?;
        assert_eq!(1, s.add.len());
        assert_eq!(1, s.remove.len());
        assert_eq!(1, e.len());

        assert_eq!(url1, s.remove.get(0).unwrap());
        assert_eq!(url2, s.add.get(0).unwrap());

        // resending the ep action to move it
        assert_eq!(15, e.get(0).unwrap().position);
        assert_eq!(url2, e.get(0).unwrap().podcast);

        Ok(())
    }

    #[test]
    fn test_upload_changes() -> Result<()> {
        let _tempfile = reset_db()?;
        let server = mock_nextcloud_server()?;
        let address = format!("http://127.0.0.1:{}", server.port());

        let login = Login {
            server: parse_url_without_scheme(&address)?,
            user: "test_user".to_string(),
            password: "test_password".to_string(),
        };

        let now = chrono::Utc::now();
        let (sub_actions, ep_actions) = make_initial_post(&now)?;
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(upload_changes(&login, sub_actions, ep_actions))?;
        Ok(())
    }

    fn mock_nextcloud_server() -> Result<TestServer> {
        let server = TestServer::new()?;

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
