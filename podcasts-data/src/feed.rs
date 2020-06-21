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


#![allow(clippy::unit_arg)]
//! Index Feeds.

use futures::future::*;
use futures::prelude::*;
use futures::stream;
use rss;

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
    pub fn index(self) -> impl Future<Item = (), Error = DataError> + Send {
        ok(self.parse_podcast())
            .and_then(|pd| pd.to_podcast())
            .and_then(move |pd| self.index_channel_items(pd))
    }

    fn parse_podcast(&self) -> NewShow {
        NewShow::new(&self.channel, self.source_id)
    }

    fn index_channel_items(self, pd: Show) -> impl Future<Item = (), Error = DataError> + Send {
        let stream = stream::iter_ok::<_, DataError>(self.channel.into_items());

        // Parse the episodes
        let episodes = stream.filter_map(move |item| {
            NewEpisodeMinimal::new(&item, pd.id())
                .and_then(move |ep| determine_ep_state(ep, &item))
                .map_err(|err| error!("Failed to parse an episode: {}", err))
                .ok()
        });

        // Filter errors, Index updatable episodes, return insertables.
        filter_episodes(episodes)
            // Batch index insertable episodes.
            .and_then(|eps| ok(batch_insert_episodes(&eps)))
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

fn filter_episodes<'a, S>(
    stream: S,
) -> impl Future<Item = Vec<NewEpisode>, Error = DataError> + Send + 'a
where
    S: Stream<Item = IndexState<NewEpisode>, Error = DataError> + Send + 'a,
{
    stream
        .filter_map(|state| match state {
            IndexState::NotChanged => None,
            // Update individual rows, and filter them
            IndexState::Update((ref ep, rowid)) => {
                ep.update(rowid)
                    .map_err(|err| error!("{}", err))
                    .map_err(|_| error!("Failed to index episode: {:?}.", ep.title()))
                    .ok();

                None
            }
            IndexState::Index(s) => Some(s),
        })
        // only Index is left, collect them for batch index
        .collect()
}

fn batch_insert_episodes(episodes: &[NewEpisode]) {
    if episodes.is_empty() {
        return;
    };

    info!("Indexing {} episodes.", episodes.len());
    dbqueries::index_new_episodes(episodes)
        .map_err(|err| {
            error!("Failed batch indexng: {}", err);
            info!("Fallign back to individual indexing.");
        })
        .unwrap_or_else(|_| {
            episodes.iter().for_each(|ep| {
                ep.index()
                    .map_err(|err| error!("Error: {}.", err))
                    .map_err(|_| error!("Failed to index episode: {:?}.", ep.title()))
                    .ok();
            });
        })
}

#[cfg(test)]
mod tests {
    use failure::Error;
    use rss::Channel;
    use tokio::{self, prelude::*};

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
        ]
    };

    #[test]
    fn test_complete_index() -> Result<(), Error> {
        truncate_db()?;

        let feeds: Vec<_> = URLS
            .iter()
            .map(|&(path, url)| {
                // Create and insert a Source into db
                let s = Source::from_url(url).unwrap();
                get_feed(path, s.id())
            })
            .collect();

        // Index the channes
        let stream_ = stream::iter_ok(feeds).for_each(|x| x.index());
        tokio::run(stream_.map_err(|_| ()));

        // Assert the index rows equal the controlled results
        assert_eq!(dbqueries::get_sources()?.len(), 5);
        assert_eq!(dbqueries::get_podcasts()?.len(), 5);
        assert_eq!(dbqueries::get_episodes()?.len(), 354);
        Ok(())
    }

    #[test]
    fn test_feed_parse_podcast() -> Result<(), Error> {
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
    fn test_feed_index_channel_items() -> Result<(), Error> {
        truncate_db()?;

        let path = "tests/feeds/2018-01-20-Intercepted.xml";
        let feed = get_feed(path, 42);
        let pd = feed.parse_podcast().to_podcast()?;

        feed.index_channel_items(pd).wait()?;
        assert_eq!(dbqueries::get_podcasts()?.len(), 1);
        assert_eq!(dbqueries::get_episodes()?.len(), 43);
        Ok(())
    }
}
