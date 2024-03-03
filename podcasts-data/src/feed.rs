// feed.rs
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

//! Index Feeds.

use crate::dbqueries;
use crate::errors::DataError;
use crate::models::{Index, IndexState, Update};
use crate::models::{NewEpisode, NewEpisodeMinimal, NewShow, Show};

/// Wrapper struct that hold a `Source` id and the `rss::Channel`
/// that corresponds to the `Source.uri` field.
#[derive(Debug, Clone, Builder, PartialEq)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub struct Feed {
    /// The `rss::Channel` parsed from the `Source` uri.
    channel: rss::Channel,
    /// The `Source` id where the xml `rss::Channel` came from.
    source_id: i32,
}

impl Feed {
    /// Index the contents of the RSS `Feed` into the database.
    pub fn index(self) -> Result<(), DataError> {
        let show = self.parse_podcast().to_podcast()?;
        self.index_channel_items(show)
    }

    fn parse_podcast(&self) -> NewShow {
        NewShow::new(&self.channel, self.source_id)
    }

    fn index_channel_items(self, pd: Show) -> Result<(), DataError> {
        let stream = self.channel.into_items().into_iter();
        // Parse the episodes
        let episodes = stream.filter_map(move |item| {
            let ret = NewEpisodeMinimal::new(&item, pd.id())
                .and_then(move |ep| determine_ep_state(ep, &item));
            if ret.is_ok() {
                Some(ret)
            } else {
                error!("importing ep: {:?}", ret);
                None
            }
        });
        // Filter errors, Index updatable episodes, return insertables.
        let insertable_episodes = filter_episodes(episodes);
        batch_insert_episodes(&insertable_episodes);
        Ok(())
    }
}

fn determine_ep_state(
    ep: NewEpisodeMinimal,
    item: &rss::Item,
) -> Result<IndexState<NewEpisode>, DataError> {
    // Check if feed exists
    let exists = dbqueries::episode_exists(ep.title(), ep.show_id())?;

    if !exists {
        Ok(IndexState::Index(ep.into_new_episode(item)))
    } else {
        let old = dbqueries::get_episode_minimal_from_pk(ep.title(), ep.show_id())?;
        let rowid = old.rowid();

        if ep != old {
            Ok(IndexState::Update((ep.into_new_episode(item), rowid)))
        } else {
            Ok(IndexState::NotChanged)
        }
    }
}

fn filter_episodes<S>(stream: S) -> Vec<NewEpisode>
where
    S: Iterator<Item = Result<IndexState<NewEpisode>, DataError>>,
{
    stream
        .filter_map(Result::ok)
        .filter_map(|state| {
            match state {
                IndexState::NotChanged => None,
                // Update individual rows, and filter them
                IndexState::Update((ref ep, rowid)) => {
                    if let Err(err) = ep.update(rowid) {
                        error!("{}", err);
                        error!("Failed to index episode: {:?}.", ep.title())
                    }
                    None
                }
                IndexState::Index(s) => Some(s),
            }
        })
        // only Index is left, collect them for batch index
        .collect()
}

fn batch_insert_episodes(episodes: &[NewEpisode]) {
    if episodes.is_empty() {
        return;
    };

    info!("Indexing {} episodes.", episodes.len());
    if let Err(err) = dbqueries::index_new_episodes(episodes) {
        error!("Failed batch indexing: {}", err);
        info!("Falling back to individual indexing.");
    } else {
        for ep in episodes {
            if let Err(err) = ep.index() {
                error!("Error: {}.", err);
                error!("Failed to index episode: {:?}.", ep.title());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rss::Channel;

    use crate::database::truncate_db;
    use crate::dbqueries;
    use crate::utils::get_feed;
    use crate::Source;

    use std::fs;
    use std::io::BufReader;

    use super::*;

    // (path, url) tuples.
    const URLS: &[(&str, &str)] = {
        &[
            (
                "tests/feeds/2018-01-20-Intercepted.xml",
                "https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.\
                 com/InterceptedWithJeremyScahill",
            ),
            (
                "tests/feeds/2018-01-20-LinuxUnplugged.xml",
                "https://web.archive.org/web/20180120110314if_/https://feeds.feedburner.\
                 com/linuxunplugged",
            ),
            (
                "tests/feeds/2018-01-20-TheTipOff.xml",
                "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff",
            ),
            (
                "tests/feeds/2018-01-20-StealTheStars.xml",
                "https://web.archive.org/web/20180120104957if_/https://rss.art19.\
                 com/steal-the-stars",
            ),
            (
                "tests/feeds/2018-01-20-GreaterThanCode.xml",
                "https://web.archive.org/web/20180120104741if_/https://www.greaterthancode.\
                 com/feed/podcast",
            ),
            (
                "tests/feeds/2022-series-i-cinema.xml",
                "https://web.archive.org/web/20220205205130_/https://dinamics.ccma.\
                 cat/public/podcast/catradio/xml/series-i-cinema.xml",
            ),
        ]
    };

    #[test]
    fn test_complete_index() -> Result<()> {
        truncate_db()?;

        let feeds: Vec<_> = URLS
            .iter()
            .map(|&(path, url)| {
                // Create and insert a Source into db
                let s = Source::from_url(url).unwrap();
                get_feed(path, s.id())
            })
            .collect();

        // Index the channels
        for feed in feeds {
            feed.index()?
        }

        // Assert the index rows equal the controlled results
        assert_eq!(dbqueries::get_sources()?.len(), 6);
        assert_eq!(dbqueries::get_podcasts()?.len(), 6);
        assert_eq!(dbqueries::get_episodes()?.len(), 404);
        Ok(())
    }

    #[test]
    fn test_feed_parse_podcast() -> Result<()> {
        truncate_db()?;

        let path = "tests/feeds/2018-01-20-Intercepted.xml";
        let feed = get_feed(path, 42);

        let file = fs::File::open(path)?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let pd = NewShow::new(&channel, 42);
        assert_eq!(feed.parse_podcast(), pd);
        Ok(())
    }

    #[test]
    fn test_feed_index_channel_items() -> Result<()> {
        truncate_db()?;

        let path = "tests/feeds/2018-01-20-Intercepted.xml";
        let feed = get_feed(path, 42);
        let pd = feed.parse_podcast().to_podcast()?;

        feed.index_channel_items(pd)?;
        assert_eq!(dbqueries::get_podcasts()?.len(), 1);
        assert_eq!(dbqueries::get_episodes()?.len(), 43);
        Ok(())
    }

    #[test]
    fn test_feed_non_utf8() -> Result<()> {
        truncate_db()?;

        let path = "tests/feeds/2022-series-i-cinema.xml";
        let feed = get_feed(path, 42);

        let file = fs::File::open(path)?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let description = feed.channel.description();
        assert_eq!(description, "Els clàssics, les novetats de la cartellera i les millors sèries, tot en un sol podcast.");
        let pd = NewShow::new(&channel, 42);
        assert_eq!(feed.parse_podcast(), pd);
        Ok(())
    }
}
