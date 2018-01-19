//! Index Feeds.

use futures::future::*;
use itertools::{Either, Itertools};
use rayon::prelude::*;
use rss;

use dbqueries;
use errors::*;
use models::{IndexState, Update};
use models::{NewEpisode, NewPodcast, Podcast};
use pipeline::*;

type InsertUpdate = (Vec<NewEpisode>, Vec<(NewEpisode, i32)>);

#[derive(Debug)]
/// Wrapper struct that hold a `Source` id and the `rss::Channel`
/// that corresponds to the `Source.uri` field.
pub struct Feed {
    channel: rss::Channel,
    source_id: i32,
}

impl Feed {
    /// Constructor that consumes a `rss::Channel`, returns a `Feed` struct.
    pub fn from_channel_source(channel: rss::Channel, source_id: i32) -> Feed {
        Feed { channel, source_id }
    }

    /// Index the contents of the RSS `Feed` into the database.
    pub fn index(&self) -> Result<()> {
        let pd = self.parse_podcast().into_podcast()?;
        self.index_channel_items(&pd)
    }

    /// Index the contents of the RSS `Feed` into the database.
    pub fn index_async(self) -> Box<Future<Item = (), Error = Error>> {
        let fut = self.parse_podcast_async()
            .and_then(|pd| pd.into_podcast())
            .and_then(move |pd| self.index_channel_items_async(&pd));

        Box::new(fut)
    }

    fn parse_podcast(&self) -> NewPodcast {
        NewPodcast::new(&self.channel, self.source_id)
    }

    fn parse_podcast_async(&self) -> Box<FutureResult<NewPodcast, Error>> {
        Box::new(ok(self.parse_podcast()))
    }

    fn index_channel_items(&self, pd: &Podcast) -> Result<()> {
        let items = self.channel.items();
        let (insert, update): (Vec<_>, Vec<_>) = items
            .into_iter()
            .filter_map(|item| glue(item, pd.id()).ok())
            .filter(|state| match *state {
                IndexState::NotChanged => false,
                _ => true,
            })
            .partition_map(|state| match state {
                IndexState::Index(e) => Either::Left(e),
                IndexState::Update(e) => Either::Right(e),
                // How not to use the unimplemented macro...
                IndexState::NotChanged => unimplemented!(),
            });

        dbqueries::index_new_episodes(insert.as_slice())?;

        update.par_iter().for_each(|&(ref ep, rowid)| {
            if let Err(err) = ep.update(rowid) {
                error!("Failed to index episode: {:?}.", ep.title());
                error!("Error msg: {}", err);
            };
        });

        Ok(())
    }

    fn index_channel_items_async(&self, pd: &Podcast) -> Box<Future<Item = (), Error = Error>> {
        let fut = self.get_stuff(pd)
            .and_then(|(insert, update)| {
                if !insert.is_empty() {
                    info!("Indexing {} episodes.", insert.len());
                    dbqueries::index_new_episodes(insert.as_slice())?;
                }
                Ok((insert, update))
            })
            .map(|(_, update)| {
                if !update.is_empty() {
                    info!("Updating {} episodes.", update.len());
                    update.iter().for_each(|&(ref ep, rowid)| {
                        if let Err(err) = ep.update(rowid) {
                            error!("Failed to index episode: {:?}.", ep.title());
                            error!("Error msg: {}", err);
                        };
                    })
                }
            });

        Box::new(fut)
    }

    fn get_stuff(&self, pd: &Podcast) -> Box<Future<Item = InsertUpdate, Error = Error>> {
        let (insert, update): (Vec<_>, Vec<_>) = self.channel
            .items()
            .into_iter()
            .map(|item| glue_async(item, pd.id()))
            .map(|fut| {
                fut.and_then(|x| match x {
                    IndexState::NotChanged => bail!("Nothing to do here."),
                    _ => Ok(x),
                })
            })
            .flat_map(|fut| fut.wait())
            .partition_map(|state| match state {
                IndexState::Index(e) => Either::Left(e),
                IndexState::Update(e) => Either::Right(e),
                // How not to use the unimplemented macro...
                IndexState::NotChanged => unimplemented!(),
            });

        Box::new(ok((insert, update)))
    }
}

#[cfg(test)]
mod tests {
    use Source;
    use database::truncate_db;
    use dbqueries;
    use pipeline;
    use std::fs;
    use std::io::BufReader;

    use super::*;

    #[test]
    /// Insert feeds and update/index them.
    fn test_index_loop() {
        truncate_db().unwrap();
        let inpt = vec![
            "https://request-for-explanation.github.io/podcast/rss.xml",
            "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
            "http://feeds.propublica.org/propublica/podcast",
            "http://feeds.feedburner.com/linuxunplugged",
        ];

        inpt.iter().for_each(|url| {
            // Index the urls into the source table.
            Source::from_url(url).unwrap();
        });
        let sources = dbqueries::get_sources().unwrap();
        pipeline::pipeline(sources, true).unwrap();

        let sources = dbqueries::get_sources().unwrap();
        // Run again to cover Unique constrains erros.
        pipeline::pipeline(sources, true).unwrap()
    }

    #[test]
    fn test_complete_index() {
        // vec of (path, url) tuples.
        let urls = vec![
            (
                "tests/feeds/Intercepted.xml",
                "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
            ),
            (
                "tests/feeds/LinuxUnplugged.xml",
                "http://feeds.feedburner.com/linuxunplugged",
            ),
            (
                "tests/feeds/TheBreakthrough.xml",
                "http://feeds.propublica.org/propublica/podcast",
            ),
            (
                "tests/feeds/R4Explanation.xml",
                "https://request-for-explanation.github.io/podcast/rss.xml",
            ),
        ];

        truncate_db().unwrap();

        let feeds: Vec<_> = urls.iter()
            .map(|&(path, url)| {
                // Create and insert a Source into db
                let s = Source::from_url(url).unwrap();

                // open the xml file
                let feed = fs::File::open(path).unwrap();
                // parse it into a channel
                let chan = rss::Channel::read_from(BufReader::new(feed)).unwrap();
                Feed::from_channel_source(chan, s.id())
            })
            .collect();

        // Index the channels
        feeds.par_iter().for_each(|x| x.index().unwrap());

        // Assert the index rows equal the controlled results
        assert_eq!(dbqueries::get_sources().unwrap().len(), 4);
        assert_eq!(dbqueries::get_podcasts().unwrap().len(), 4);
        assert_eq!(dbqueries::get_episodes().unwrap().len(), 274);
    }
}
