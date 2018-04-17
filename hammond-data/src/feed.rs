//! Index Feeds.

use futures::future::*;
use rss;

use dbqueries;
use errors::DataError;
use models::{Index, IndexState, Update};
use models::{NewPodcast, Podcast};
use pipeline::*;

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
            .and_then(move |pd| self.index_channel_items(&pd));

        Box::new(fut)
    }

    fn parse_podcast(&self) -> NewPodcast {
        NewPodcast::new(&self.channel, self.source_id)
    }

    fn parse_podcast_async(&self) -> Box<Future<Item = NewPodcast, Error = DataError> + Send> {
        Box::new(ok(self.parse_podcast()))
    }

    fn index_channel_items(
        &self,
        pd: &Podcast,
    ) -> Box<Future<Item = (), Error = DataError> + Send> {
        Box::new(ok(self.index_stuff(pd)))
    }

    fn index_stuff(&self, pd: &Podcast) {
        let insert: Vec<_> = self.channel
            .items()
            .iter()
            // FIXME: print the error
            .filter_map(|item| glue(item, pd.id()).ok())
            .filter_map(|state| match state {
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

        if !insert.is_empty() {
            info!("Indexing {} episodes.", insert.len());
            if let Err(err) = dbqueries::index_new_episodes(insert.as_slice()) {
                error!("Failed batch indexng, Fallign back to individual indexing.");
                error!("{}", err);

                insert.iter().for_each(|ep| {
                    if let Err(err) = ep.index() {
                        error!("Failed to index episode: {:?}.", ep.title());
                        error!("{}", err);
                    };
                })
            }
        }
    }
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

        feed.index_channel_items(&pd).wait().unwrap();
        assert_eq!(dbqueries::get_podcasts().unwrap().len(), 1);
        assert_eq!(dbqueries::get_episodes().unwrap().len(), 43);
    }
}
