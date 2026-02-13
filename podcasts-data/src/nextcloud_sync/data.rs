// nextcloud_sync/data.rs
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

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use reqwest;
use reqwest::Url;
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

/// A date stored as UTC+0 seconds since 1970
type UnixTime = i64;
/// A number where -1 means None
type OptionalNumber = i32;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Url parse error.")]
    UrlParseError(#[from] url::ParseError),
    #[error("Unexpected Request response: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Data Error.")]
    DataError(#[from] crate::errors::DataError),
    #[error("Error.")]
    AnyhowError(#[from] anyhow::Error),

    #[error("Unexpected server response: {0}")]
    UnexpectedResponse(reqwest::StatusCode),
    #[error("Downloaded update for episode that is not in db.")]
    DownloadedUpdateForEpisodeNotInDb,
    #[error("API not found, GPodder Sync extension may not be active on the Nextcloud Server.")]
    NoSubscriptionApi,
}

#[derive(PartialEq, Debug)]
/// Stats about what was synced
pub enum SyncResult {
    Done {
        /// How many episode updates were applied from remote
        episode_updates_downloaded: usize,
        /// How many subscription updates were applied from remote
        subscription_updates_downloaded: usize,
    },
    /// The sync was skipped, because there were no credentials, or it was turned off
    Skipped,
}

/// Response for downloading Podcast subscription updates.
#[derive(Deserialize, Debug)]
pub(crate) struct SubscriptionGet {
    /// new subscriptions (as url strings)
    pub(crate) add: Vec<String>,
    /// podcasts the user unsubscribed from (as url strings)
    pub(crate) remove: Vec<String>,
    /// Time of the change-set
    pub(crate) timestamp: UnixTime,
}

/// An episode Update that we send to the Server.
#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct SubscriptionPost {
    /// new subscriptions (as url strings)
    pub(crate) add: Vec<String>,
    /// podcasts the user unsubscribed from (as url strings)
    pub(crate) remove: Vec<String>,
}

impl SubscriptionPost {
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.remove.is_empty()
    }

    pub fn remove_already_on_server(&mut self, sub_get: &SubscriptionGet) {
        let add = std::mem::take(&mut self.add);
        let remove = std::mem::take(&mut self.remove);
        self.add = add
            .into_iter()
            .filter(|uri| !sub_get.add.contains(uri))
            .collect();
        self.remove = remove
            .into_iter()
            .filter(|uri| !sub_get.remove.contains(uri))
            .collect();
    }
}

/// Response for downloading episode updates.
#[allow(unused)]
#[derive(Deserialize, Debug)]
pub(crate) struct EpisodeGet {
    pub(crate) actions: Vec<EpisodeAction>,
    pub(crate) timestamp: UnixTime,
}

/// Data required for a login.
pub(crate) struct Login {
    /// Address where NextCloud is hosted. e.g.: https://cloud.example.com
    pub(crate) server: Url,
    /// Username for the login.
    pub(crate) user: String,
    /// App specific password obtained from loginFlow see login.rs.
    pub(crate) password: String,
}

/// https://gpoddernet.readthedocs.io/en/latest/api/reference/events.html
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Action {
    /// Tell other devices that this was aleady downloaded.
    Download,
    /// Tell other devices that this file is no longer downloaded.
    Delete,
    /// Tell other devices that this episode was played to a specific position.
    Play,
    /// New could also be called "Reset"
    /// Clients can send `New` states to reset previous events.
    /// This state needs to be interpreted by receiving clients and does not delete any information on the webservice.
    New,
    /// old donation service
    Flattr,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Action::Download => "DOWNLOAD",
                Action::Delete => "DELETE",
                Action::Play => "PLAY",
                Action::New => "NEW",
                Action::Flattr => "FLATTR",
            }
        )
    }
}

impl FromStr for Action {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_uppercase();
        match s.as_str() {
            "DOWNLOAD" => Ok(Action::Download),
            "DELETE" => Ok(Action::Delete),
            "PLAY" => Ok(Action::Play),
            "NEW" => Ok(Action::New),
            "FLATTR" => Ok(Action::Flattr),
            _ => bail!("failed to deserialize gpodder-Action"),
        }
    }
}

impl Serialize for Action {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}
impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(deserializer)?;
        Action::from_str(&buf).map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct EpisodeAction {
    pub(crate) podcast: String,
    pub(crate) episode: String,
    pub(crate) guid: Option<String>,
    pub(crate) action: Action,
    #[serde(serialize_with = "to_iso")]
    #[serde(deserialize_with = "from_iso")]
    pub(crate) timestamp: DateTime<Utc>,
    // Only valid for “play”. the position (in seconds) at which the client started playback. Requires position and total to be set.
    pub(crate) started: OptionalNumber,  // where PLAY started
    pub(crate) position: OptionalNumber, // where PLAY ended
    pub(crate) total: OptionalNumber,    // total file duration
}

fn to_iso<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // For Old versions:
    // NOT ACTUALLY ISO RFC3339 format, CAN NOT HANDLE TIMEZONE DATA
    serializer.serialize_str(&format!("{}", dt.format("%Y-%m-%dT%H:%M:%S")))
}
fn from_iso<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    // Old format without Timestamp. Kept around in case it will be useful
    // for Gpodder Server Sync, or old nextcloud versions for now.
    let buf = String::deserialize(deserializer)?;
    let naive = chrono::NaiveDateTime::parse_from_str(&buf, "%Y-%m-%dT%H:%M:%S");
    naive
        .map(|d| chrono::DateTime::from_naive_utc_and_offset(d, chrono::Utc))
        .or_else(|_| {
            // New versions use rfc3339 dates.
            chrono::DateTime::parse_from_rfc3339(&buf).map(|dt| dt.with_timezone(&chrono::Utc))
        })
        .map_err(serde::de::Error::custom)
}

impl EpisodeAction {
    /// Determines if the play time is close to the end of the episode.
    /// calculation notes from:
    /// https://gitlab.gnome.org/World/podcasts/-/issues/66
    pub(crate) fn finished_play(&self) -> bool {
        // TODO adjust this more
        let diff = self.total - self.position;
        // special timing for short episodes
        // 420 = 7 min
        if self.total < 420 {
            return diff < 35; //  less than 35s remain
        }
        // some eps play a 1:30m music outro
        // less than 90s or 5% of the file remain
        (self.total - self.position < 90) || (self.position as f32) > (self.total as f32 / 0.05)
    }

    /// Remove Play Actions that are behind the ones on the server.
    pub(crate) fn already_on_server(&self, ep_get: &EpisodeGet) -> bool {
        if self.action == Action::Play {
            ep_get
                .actions
                .iter()
                .any(|e| self.is_same_episode(e) && self.position <= e.position)
        } else {
            false
        }
    }

    /// Has the same episode uri or guid and it's not emtpy.
    pub(crate) fn is_same_episode(&self, other: &EpisodeAction) -> bool {
        (!self.episode.is_empty() && self.episode == other.episode)
            || (self.guid.is_some() && self.guid == other.guid)
    }
}

pub(crate) fn parse_url_without_scheme(s: &str) -> Result<Url, url::ParseError> {
    Url::parse(s).or(Url::parse(&["https://", s].join("")))
}

pub(crate) fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder().user_agent(crate::USER_AGENT_NEXTCLOUD)
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::test_feeds::*;
    use http_test_server::TestServer;
    use std::sync::LazyLock;

    pub fn actions_response(server: &TestServer) -> String {
        let template = include_str!("../../tests/sync/nextcloud_actions_template.json");
        let server_url = format!("http://127.0.0.1:{}", server.port());
        template.replace("%TEST_SERVER%", &server_url)
    }

    pub fn actions_with_missing_sub(server: &TestServer) -> String {
        let template =
            include_str!("../../tests/sync/nextcloud_actions_with_missing_sub_template.json");
        let server_url = format!("http://127.0.0.1:{}", server.port());
        template.replace("%TEST_SERVER%", &server_url)
    }

    static RUNTIME: LazyLock<tokio::runtime::Runtime> =
        LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());

    pub fn prepare() -> Result<(&'static tokio::runtime::Runtime, tempfile::NamedTempFile)> {
        let tempfile = crate::database::reset_db()?;
        let _ = crate::feed_manager::RUNTIME.set(&RUNTIME);
        Ok((&RUNTIME, tempfile))
    }

    #[test]
    fn test_parse_subs() {
        let s = "{\
    \"add\": [\
        \"https://media.rss.com/the-antifada/feed.xml\",\
        \"https://feeds.libsyn.com/399887/rss\",\
        \"https://feed.syntax.fm/rss\",\
        \"https://audioboom.com/channels/5094626.rss\",\
        \"https://feeds.soundcloud.com/users/soundcloud:users:492135420/sounds.rss\",\
        \"https://feeds.soundcloud.com/users/soundcloud:users:211911700/sounds.rss\",\
        \"https://api.substack.com/feed/podcast/28705.rss\",\
        \"https://anchor.fm/s/4d855a8c/podcast/rss\",\
        \"https://feeds.libsyn.com/152597/rss\",\
        \"https://feeds.soundcloud.com/users/soundcloud:users:125332894/sounds.rss\",\
        \"https://anchor.fm/s/3b394974/podcast/rss\",\
        \"https://rustacean-station.org/podcast.rss\",\
        \"https://rss.art19.com/the-deprogram\",\
        \"http://faif.us/feeds/cast-ogg/\"\
    ],\
    \"remove\": [],\
    \"timestamp\": 1691947383\
    }";

        let e: Option<SubscriptionGet> = serde_json::from_str(s).ok();
        assert!(e.is_some());
    }

    #[test]
    fn test_parse_ep_actions() -> Result<()> {
        let server = mock_feed_server()?;
        let s = actions_response(&server);
        let _: EpisodeGet = serde_json::from_str(&s).unwrap();
        Ok(())
    }

    #[test]
    fn finished_play() {
        // unset, wrong action type
        let ea = EpisodeAction {
            podcast: "test".to_string(),
            episode: "test".to_string(),
            guid: Some("test".to_string()),
            action: Action::Download,
            timestamp: chrono::Utc::now(),
            started: -1,
            position: -1,
            total: -1,
        };
        assert!(ea.finished_play());
        // middle of a short ep
        let ea = EpisodeAction {
            podcast: "test".to_string(),
            episode: "test".to_string(),
            guid: Some("test".to_string()),
            action: Action::Play,
            timestamp: chrono::Utc::now(),
            started: 0,
            position: 200,
            total: 419,
        };
        assert!(!ea.finished_play());
        // more than 35s remain
        let ea = EpisodeAction {
            podcast: "test".to_string(),
            episode: "test".to_string(),
            guid: Some("test".to_string()),
            action: Action::Play,
            timestamp: chrono::Utc::now(),
            started: 0,
            position: 380,
            total: 419,
        };
        assert!(!ea.finished_play());
        // less than 35s remain
        let ea = EpisodeAction {
            podcast: "test".to_string(),
            episode: "test".to_string(),
            guid: Some("test".to_string()),
            action: Action::Play,
            timestamp: chrono::Utc::now(),
            started: 0,
            position: 390,
            total: 419,
        };
        assert!(ea.finished_play());
        let ea = EpisodeAction {
            podcast: "test".to_string(),
            episode: "test".to_string(),
            guid: Some("test".to_string()),
            action: Action::Play,
            timestamp: chrono::Utc::now(),
            started: 0,
            position: 800,
            total: 1000,
        };
        assert!(!ea.finished_play());
        // less than 5%
        let ea = EpisodeAction {
            podcast: "test".to_string(),
            episode: "test".to_string(),
            guid: Some("test".to_string()),
            action: Action::Play,
            timestamp: chrono::Utc::now(),
            started: 0,
            position: 960,
            total: 1000,
        };
        assert!(ea.finished_play());
        // 100%
        let ea = EpisodeAction {
            podcast: "test".to_string(),
            episode: "test".to_string(),
            guid: Some("test".to_string()),
            action: Action::Play,
            timestamp: chrono::Utc::now(),
            started: 0,
            position: 1000,
            total: 1000,
        };
        assert!(ea.finished_play());
    }

    #[test]
    fn test_parse_url_without_scheme() -> Result<()> {
        let url = parse_url_without_scheme("example.com")?;
        assert_eq!("https://example.com/", url.to_string().as_str());
        let url = parse_url_without_scheme("http://example.com")?;
        assert_eq!("http://example.com/", url.to_string().as_str());
        let url = parse_url_without_scheme("http://example.com/")?;
        assert_eq!("http://example.com/", url.to_string().as_str());
        Ok(())
    }

    #[test]
    fn test_already_on_server() -> Result<()> {
        let ea = EpisodeAction {
            podcast: "test".to_string(),
            episode: "test".to_string(),
            guid: Some("test".to_string()),
            action: Action::Play,
            timestamp: chrono::Utc::now(),
            started: 0,
            position: 960,
            total: 1000,
        };
        let server_actions = EpisodeGet {
            actions: vec![ea.clone()],
            timestamp: 0,
        };

        assert!(ea.already_on_server(&server_actions));

        let ea2 = EpisodeAction {
            podcast: "test2".to_string(),
            episode: "test2".to_string(),
            guid: Some("test2".to_string()),
            action: Action::Play,
            timestamp: chrono::Utc::now(),
            started: 0,
            position: 960,
            total: 1000,
        };

        assert!(!ea2.already_on_server(&server_actions));
        Ok(())
    }
}
