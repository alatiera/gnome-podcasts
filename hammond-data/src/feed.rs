//! Index Feeds.

use futures::future::*;
use futures::prelude::*;
use futures::stream;
use rss;

use dbqueries;
use errors::DataError;
use models::{Index, IndexState, Update};
use models::{NewEpisode, NewEpisodeMinimal, NewPodcast, Podcast};

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
    pub fn index(self) -> Box<Future<Item = (), Error = DataError> + Send> {
        let fut = self.parse_podcast_async()
            .and_then(|pd| pd.to_podcast())
            .and_then(move |pd| self.index_channel_items(pd));

        Box::new(fut)
    }

    fn parse_podcast(&self) -> NewPodcast {
        NewPodcast::new(&self.channel, self.source_id)
    }

    fn parse_podcast_async(&self) -> Box<Future<Item = NewPodcast, Error = DataError> + Send> {
        Box::new(ok(self.parse_podcast()))
    }

    fn index_channel_items(self, pd: Podcast) -> Box<Future<Item = (), Error = DataError> + Send> {
        let stream = stream::iter_ok::<_, DataError>(self.channel.items_owned());

        // Parse the episodes
        let episodes = stream.filter_map(move |item| {
            glue(&item, pd.id())
                .map_err(|err| error!("Failed to parse an episode: {}", err))
                .ok()
        });

        // Filter errors, Index updatable episodes, return insertables.
        let insertables = filter_episodes(episodes);
        // Batch index insertable episodes.
        let idx = insertables.and_then(|eps| ok(batch_insert_episodes(eps)));

        Box::new(idx)
    }
}

fn glue(item: &rss::Item, id: i32) -> Result<IndexState<NewEpisode>, DataError> {
    NewEpisodeMinimal::new(item, id).and_then(move |ep| determine_ep_state(ep, item))
}

fn determine_ep_state(
    ep: NewEpisodeMinimal,
    item: &rss::Item,
) -> Result<IndexState<NewEpisode>, DataError> {
    // Check if feed exists
    let exists = dbqueries::episode_exists(ep.title(), ep.podcast_id())?;

    if !exists {
        Ok(IndexState::Index(ep.into_new_episode(item)))
    } else {
        let old = dbqueries::get_episode_minimal_from_pk(ep.title(), ep.podcast_id())?;
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
) -> Box<Future<Item = Vec<NewEpisode>, Error = DataError> + Send + 'a>
where
    S: Stream<Item = IndexState<NewEpisode>, Error = DataError> + Send + 'a,
{
    let list = stream.filter_map(|state| match state {
        IndexState::NotChanged => None,
        // Update individual rows, and filter them
        IndexState::Update((ref ep, rowid)) => {
            ep.update(rowid)
                .map_err(|err| error!("{}", err))
                .map_err(|_| error!("Failed to index episode: {:?}.", ep.title()))
                .ok();

            None
        },
        IndexState::Index(s) => Some(s),
    })
    // only Index is left, collect them for batch index
    .collect();

    Box::new(list)
}

fn batch_insert_episodes(episodes: Vec<NewEpisode>) {
    if episodes.is_empty() {
        return;
    };

    info!("Indexing {} episodes.", episodes.len());
    dbqueries::index_new_episodes(episodes.as_slice())
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
    use rss::Channel;
    use tokio_core::reactor::Core;

    use database::truncate_db;
    use dbqueries;
    use utils::get_feed;
    use Source;

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
    fn test_complete_index() {
        truncate_db().unwrap();

        let feeds: Vec<_> = URLS.iter()
            .map(|&(path, url)| {
                // Create and insert a Source into db
                let s = Source::from_url(url).unwrap();
                get_feed(path, s.id())
            })
            .collect();

        let mut core = Core::new().unwrap();
        // Index the channels
        let list: Vec<_> = feeds.into_iter().map(|x| x.index()).collect();
        let _foo = core.run(join_all(list));

        // Assert the index rows equal the controlled results
        assert_eq!(dbqueries::get_sources().unwrap().len(), 5);
        assert_eq!(dbqueries::get_podcasts().unwrap().len(), 5);
        assert_eq!(dbqueries::get_episodes().unwrap().len(), 354);
    }

    #[test]
    fn test_feed_parse_podcast() {
        truncate_db().unwrap();

        let path = "tests/feeds/2018-01-20-Intercepted.xml";
        let feed = get_feed(path, 42);

        let file = fs::File::open(path).unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let pd = NewPodcast::new(&channel, 42);
        assert_eq!(feed.parse_podcast(), pd);
    }

    #[test]
    fn test_feed_index_channel_items() {
        truncate_db().unwrap();

        let path = "tests/feeds/2018-01-20-Intercepted.xml";
        let feed = get_feed(path, 42);
        let pd = feed.parse_podcast().to_podcast().unwrap();

        feed.index_channel_items(pd).wait().unwrap();
        assert_eq!(dbqueries::get_podcasts().unwrap().len(), 1);
        assert_eq!(dbqueries::get_episodes().unwrap().len(), 43);
    }
}
