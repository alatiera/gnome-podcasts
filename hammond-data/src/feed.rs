//! Index Feeds.

use futures::future::*;
use itertools::{Either, Itertools};
use rss;

use dbqueries;
use errors::DataError;
use models::{Index, IndexState, Update};
use models::{NewEpisode, NewPodcast, Podcast};
use pipeline::*;

type InsertUpdate = (Vec<NewEpisode>, Vec<Option<(NewEpisode, i32)>>);

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
        let fut = self.get_stuff(pd)
            .and_then(|(insert, update)| {
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
                Ok((insert, update))
            })
            .map(|(_, update)| {
                if !update.is_empty() {
                    info!("Updating {} episodes.", update.len());
                    // see get_stuff for more
                    update
                        .into_iter()
                        .filter_map(|x| x)
                        .for_each(|(ref ep, rowid)| {
                            if let Err(err) = ep.update(rowid) {
                                error!("Failed to index episode: {:?}.", ep.title());
                                error!("{}", err);
                            };
                        })
                }
            });

        Box::new(fut)
    }

    fn get_stuff(
        &self,
        pd: &Podcast,
    ) -> Box<Future<Item = InsertUpdate, Error = DataError> + Send> {
        let (insert, update): (Vec<_>, Vec<_>) = self.channel
            .items()
            .into_iter()
            .map(|item| glue_async(item, pd.id()))
            // This is sort of ugly but I think it's cheaper than pushing None
            // to updated and filtering it out later.
            // Even though we already map_filter in index_channel_items.
            // I am not sure what the optimizations are on match vs allocating None.
            .map(|fut| {
                fut.and_then(|x| match x {
                    IndexState::NotChanged => return Err(DataError::EpisodeNotChanged),
                    _ => Ok(x),
                })
            })
            .flat_map(|fut| fut.wait())
            .partition_map(|state| match state {
                IndexState::Index(e) => Either::Left(e),
                IndexState::Update(e) => Either::Right(Some(e)),
                // This should never occur
                IndexState::NotChanged => Either::Right(None),
            });

        Box::new(ok((insert, update)))
    }
}

#[cfg(test)]
mod tests {
    use rss::Channel;
    use tokio_core::reactor::Core;

    use Source;
    use database::truncate_db;
    use dbqueries;
    use utils::get_feed;

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

    #[test]
    fn test_feed_get_stuff() {
        truncate_db().unwrap();

        let path = "tests/feeds/2018-01-20-Intercepted.xml";
        let feed = get_feed(path, 42);
        let pd = feed.parse_podcast().to_podcast().unwrap();

        let (insert, update) = feed.get_stuff(&pd).wait().unwrap();
        assert_eq!(43, insert.len());
        assert_eq!(0, update.len());

        feed.index().wait().unwrap();

        let path = "tests/feeds/2018-02-03-Intercepted.xml";
        let feed = get_feed(path, 42);
        let pd = feed.parse_podcast().to_podcast().unwrap();

        let (insert, update) = feed.get_stuff(&pd).wait().unwrap();
        assert_eq!(4, insert.len());
        assert_eq!(43, update.len());
    }
}
