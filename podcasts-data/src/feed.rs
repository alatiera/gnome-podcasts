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
use crate::models::{EpisodeId, NewEpisode, NewEpisodeMinimal, NewShow, Show, SourceId};
use crate::models::{Index, IndexState, Update};

/// Wrapper struct that hold a `Source` id and the `rss::Channel`
/// that corresponds to the `Source.uri` field.
#[derive(Debug, Clone, Builder, PartialEq)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub struct Feed {
    /// The `rss::Channel` parsed from the `Source` uri.
    channel: rss::Channel,
    /// The `Source` id where the xml `rss::Channel` came from.
    source_id: SourceId,
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
) -> Result<IndexState<NewEpisode, EpisodeId>, DataError> {
    // Check if feed exists
    let exists = dbqueries::episode_exists(ep.guid(), ep.title(), ep.show_id())?;

    if !exists {
        Ok(IndexState::Index(ep.into_new_episode(item)))
    } else {
        let old = dbqueries::get_episode_minimal(ep.guid(), ep.title(), ep.show_id())?;
        let id = old.id();

        if ep != old {
            Ok(IndexState::Update((ep.into_new_episode(item), id)))
        } else {
            Ok(IndexState::NotChanged)
        }
    }
}

fn filter_episodes<S>(stream: S) -> Vec<NewEpisode>
where
    S: Iterator<Item = Result<IndexState<NewEpisode, EpisodeId>, DataError>>,
{
    let result: Vec<NewEpisode> = stream
        .filter_map(Result::ok)
        .filter_map(|state| {
            match state {
                IndexState::NotChanged => None,
                // Update individual rows, and filter them
                IndexState::Update((ref ep, id)) => {
                    if let Err(err) = ep.update(id) {
                        error!("{}", err);
                        error!("Failed to index episode: {:?}.", ep.title())
                    }
                    None
                }
                IndexState::Index(s) => Some(s),
            }
        })
        // only Index is left, collect them for batch index
        .collect();

    // filter out duplicates with same guid or title, they are assumed to be the same episode
    let mut set = std::collections::HashSet::new();
    result
        .into_iter()
        .filter(|ep| {
            let id = ep.guid().unwrap_or(ep.title()).to_string();
            set.insert(id)
        })
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

    use crate::EpisodeModel;
    use crate::Source;
    use crate::database::truncate_db;
    use crate::dbqueries;
    use crate::utils::get_feed;

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
    /// randomly chosen
    const TEST_SOURCE_ID: SourceId = SourceId(42);

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
        let feed = get_feed(path, TEST_SOURCE_ID);

        let file = fs::File::open(path)?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let pd = NewShow::new(&channel, TEST_SOURCE_ID);
        assert_eq!(feed.parse_podcast(), pd);
        Ok(())
    }

    #[test]
    fn test_feed_index_channel_items() -> Result<()> {
        truncate_db()?;

        let path = "tests/feeds/2018-01-20-Intercepted.xml";
        let feed = get_feed(path, TEST_SOURCE_ID);
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
        let feed = get_feed(path, TEST_SOURCE_ID);

        let file = fs::File::open(path)?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let description = feed.channel.description();
        assert_eq!(
            description,
            "Els clàssics, les novetats de la cartellera i les millors sèries, tot en un sol podcast."
        );
        let pd = NewShow::new(&channel, TEST_SOURCE_ID);
        assert_eq!(feed.parse_podcast(), pd);
        Ok(())
    }

    // https://gitlab.gnome.org/World/podcasts/-/issues/239
    #[test]
    fn test_feed_same_title_different_guid() -> Result<()> {
        truncate_db()?;

        let path = "tests/feeds/de-grote.xml";
        let feed = get_feed(path, TEST_SOURCE_ID);
        let pd = feed.parse_podcast().to_podcast()?;

        feed.index_channel_items(pd)?;
        assert_eq!(dbqueries::get_podcasts()?.len(), 1);
        assert_eq!(dbqueries::get_episodes()?.len(), 12);
        Ok(())
    }

    // https://gitlab.gnome.org/World/podcasts/-/issues/239
    #[test]
    fn test_feed_same_title_no_guid() -> Result<()> {
        truncate_db()?;

        let path = "tests/feeds/de-grote-no-guid.xml";
        let feed = get_feed(path, TEST_SOURCE_ID);
        let pd = feed.parse_podcast().to_podcast()?;

        feed.index_channel_items(pd)?;

        let eps = dbqueries::get_episodes()?;

        assert_eq!(1, dbqueries::get_podcasts()?.len());
        assert_eq!(2, eps.len());

        // latest episode (latest item in feed), previous items with same title are ignored
        let ep1 = eps.get(0).unwrap();
        assert_eq!(
            Some(
                "https://chtbl.com/track/11G3D/progressive-audio.vrt.be/public/output/aud-7478134e-7c0e-44d4-8d65-32aa87dc6a3a-PODCAST_1/aud-7478134e-7c0e-44d4-8d65-32aa87dc6a3a-PODCAST_1.mp3"
            ),
            ep1.uri()
        );

        // teaser (first item in feed)
        let ep2 = eps.get(1).unwrap();
        assert_eq!(
            Some(
                "https://chtbl.com/track/11G3D/progressive-audio.vrt.be/public/output/aud-6b925160-4400-4d50-bb54-0085b84643cd-PODCAST_1/aud-6b925160-4400-4d50-bb54-0085b84643cd-PODCAST_1.mp3"
            ),
            ep2.uri()
        );

        Ok(())
    }

    // https://gitlab.gnome.org/World/podcasts/-/issues/204
    #[test]
    fn test_reruns() -> Result<()> {
        truncate_db()?;

        let path = "tests/feeds/2020-12-29-replyall.xml";
        let feed = get_feed(path, TEST_SOURCE_ID);
        let pd = feed.parse_podcast().to_podcast()?;
        feed.index_channel_items(pd)?;
        let show_id = dbqueries::get_podcasts()?.get(0).unwrap().id();

        let eps = dbqueries::get_episodes()?;
        let rerun_eps: Vec<_> = eps
            .into_iter()
            .filter(|e| e.title() == "#86 Man of the People")
            .collect();

        assert_eq!(rerun_eps.len(), 2);
        // rerun
        let ep1 = rerun_eps.get(0).unwrap();
        assert_eq!("#86 Man of the People", ep1.title());
        assert_eq!(Some("c16006fa-e2c3-11e9-be80-bf4954f39568"), ep1.guid());
        assert_eq!(
            Some("https://traffic.megaphone.fm/GLT8202680871.mp3?updated=1607019082"),
            ep1.uri()
        );
        assert_eq!(
            &dbqueries::get_episode(
                Some("c16006fa-e2c3-11e9-be80-bf4954f39568"),
                "#86 Man of the People",
                show_id
            )?,
            ep1
        );

        // original run
        let ep2 = rerun_eps.get(1).unwrap();
        assert_eq!("#86 Man of the People", ep2.title());
        assert_eq!(Some("3e7f1804-affc-11e6-892a-bb965a8b4a3f"), ep2.guid());
        assert_eq!(
            Some("https://traffic.megaphone.fm/GLT1103232835.mp3?updated=1486920888"),
            ep2.uri()
        );
        assert_eq!(
            &dbqueries::get_episode(
                Some("3e7f1804-affc-11e6-892a-bb965a8b4a3f"),
                "#86 Man of the People",
                show_id
            )?,
            ep2
        );

        Ok(())
    }

    #[test]
    // has same title and &amp; sign in title
    fn test_same_title_streetfight() -> Result<()> {
        truncate_db()?;

        let path = "tests/feeds/2024-03-15-streetfightradio.xml";
        let feed = get_feed(path, TEST_SOURCE_ID);
        let pd = feed.parse_podcast().to_podcast()?;
        feed.index_channel_items(pd)?;
        let show_id = dbqueries::get_podcasts()?.get(0).unwrap().id();

        let eps = dbqueries::get_episodes()?;
        let same_title_eps: Vec<_> = eps
            .clone()
            .into_iter()
            .filter(|e| e.title() == "Return Of The Macks")
            .collect();

        assert_eq!(2, same_title_eps.len());
        let ep1 = same_title_eps.get(0).unwrap();
        assert_eq!("Return Of The Macks", ep1.title());
        assert_eq!(Some("tag:soundcloud,2010:tracks/501720369"), ep1.guid());
        assert_eq!(
            Some(
                "https://feeds.soundcloud.com/stream/501720369-streetfightwcrs-return-of-the-macks-1.mp3"
            ),
            ep1.uri()
        );
        assert_eq!(
            &dbqueries::get_episode(
                Some("tag:soundcloud,2010:tracks/501720369"),
                "Return Of The Macks",
                show_id
            )?,
            ep1
        );

        let ep2 = same_title_eps.get(1).unwrap();
        assert_eq!("Return Of The Macks", ep2.title());
        assert_eq!(Some("tag:soundcloud,2010:tracks/430832790"), ep2.guid());
        assert_eq!(
            Some(
                "https://feeds.soundcloud.com/stream/430832790-streetfightwcrs-return-of-the-macks.mp3"
            ),
            ep2.uri()
        );
        assert_eq!(
            &dbqueries::get_episode(
                Some("tag:soundcloud,2010:tracks/430832790"),
                "Return Of The Macks",
                show_id
            )?,
            ep2
        );

        // second title
        let same_title_eps: Vec<_> = eps
            .into_iter()
            .filter(|e| e.title() == "Street Fight Q&A")
            .collect();

        assert_eq!(2, same_title_eps.len());
        let ep1 = same_title_eps.get(0).unwrap();
        assert_eq!("Street Fight Q&A", ep1.title());
        assert_eq!(Some("tag:soundcloud,2010:tracks/658646834"), ep1.guid());
        assert_eq!(
            Some(
                "https://feeds.soundcloud.com/stream/658646834-streetfightwcrs-street-fight-qa-1.mp3"
            ),
            ep1.uri()
        );
        assert_eq!(
            &dbqueries::get_episode(
                Some("tag:soundcloud,2010:tracks/658646834"),
                "Street Fight Q&A",
                show_id
            )?,
            ep1
        );

        let ep2 = same_title_eps.get(1).unwrap();
        assert_eq!("Street Fight Q&A", ep2.title());
        assert_eq!(Some("tag:soundcloud,2010:tracks/624834786"), ep2.guid());
        assert_eq!(
            Some(
                "https://feeds.soundcloud.com/stream/624834786-streetfightwcrs-street-fight-qa.mp3"
            ),
            ep2.uri()
        );
        assert_eq!(
            &dbqueries::get_episode(
                Some("tag:soundcloud,2010:tracks/624834786"),
                "Street Fight Q&A",
                show_id
            )?,
            ep2
        );

        Ok(())
    }
}
