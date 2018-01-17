//! Index Feeds.

use rayon::prelude::*;
use diesel::prelude::*;
use futures::prelude::*;
use futures::future::*;
use rayon::iter::IntoParallelIterator;

use rss;

use dbqueries;
use parser;

use models::queryables::{Podcast, Source};
use models::insertables::{NewEpisode, NewPodcast};
use database::connection;
use errors::*;

// #[cfg(test)]
// use models::queryables::Episode;

#[derive(Debug)]
/// Wrapper struct that hold a `Source` id and the `rss::Channel`
/// that corresponds to the `Source.uri` field.
pub struct Feed {
    channel: rss::Channel,
    source_id: i32,
}

impl Feed {
    /// Constructor that consumes a `Source` and returns the corresponding `Feed` struct.
    pub fn from_source(s: &mut Source) -> Result<Feed> {
        s.into_feed(false)
    }

    /// Constructor that consumes a `Source` and a `rss::Channel` returns a `Feed` struct.
    pub fn from_channel_source(channel: rss::Channel, source_id: i32) -> Feed {
        Feed { channel, source_id }
    }

    /// Index the contents of the RSS `Feed` into the database.
    pub fn index(&self) -> Result<()> {
        let pd = self.parse_podcast().into_podcast()?;
        self.index_channel_items(&pd)
    }

    /// Docs
    // FIXME: docs
    // FIXME: lifetime stuff
    pub fn index_future(self) -> Box<Future<Item = (), Error = Error>> {
        let indx = self.parse_podcast_futture()
            .and_then(|pd| pd.into_podcast())
            .and_then(move |pd| self.index_channel_items(&pd));

        Box::new(indx)
    }

    // TODO: Refactor transcactions and find a way to do it in parallel.
    fn index_channel_items(&self, pd: &Podcast) -> Result<()> {
        let episodes = self.parse_channel_items(pd);
        let db = connection();
        let con = db.get()?;

        let _ = con.transaction::<(), Error, _>(|| {
            episodes.into_iter().for_each(|x| {
                if let Err(err) = x.index(&con) {
                    error!("Failed to index episode: {:?}.", x.title());
                    error!("Error msg: {}", err);
                };
            });
            Ok(())
        });
        Ok(())
    }

    fn parse_podcast(&self) -> NewPodcast {
        parser::new_podcast(&self.channel, self.source_id)
    }

    fn parse_podcast_futture(&self) -> Box<FutureResult<NewPodcast, Error>> {
        Box::new(ok(self.parse_podcast()))
    }

    fn parse_channel_items(&self, pd: &Podcast) -> Vec<NewEpisode> {
        let items = self.channel.items();
        let new_episodes: Vec<_> = items
            .par_iter()
            .filter_map(|item| NewEpisode::new(item, pd.id()).ok())
            .collect();

        new_episodes
    }

    // This could also retrurn a FutureResult<Vec<FutureNewEpisode, Error>>, Error> Instead
    #[allow(dead_code)]
    fn parse_episodes_future(&self, pd: &Podcast) -> Box<Vec<FutureResult<NewEpisode, Error>>> {
        let episodes = self.channel
            .items()
            .par_iter()
            .map(|item| result(NewEpisode::new(item, pd.id())))
            .collect();

        Box::new(episodes)
    }

    // #[cfg(test)]
    // /// This returns only the episodes in the xml feed.
    // fn get_episodes(&self) -> Result<Vec<Episode>> {
    //     let pd = self.get_podcast()?;
    //     let eps = self.parse_channel_items(&pd);

    //     let db = connection();
    //     let con = db.get()?;
    //     let episodes: Vec<_> = eps.into_iter()
    //         .filter_map(|ep| ep.into_episode(&con).ok())
    //         .collect();

    //     Ok(episodes)
    // }
}

/// Index a "list" of `Source`s.
pub fn index_loop<S: IntoParallelIterator<Item = Source>>(sources: S) {
    sources
        .into_par_iter()
        .filter_map(|mut source| {
            let foo = Feed::from_source(&mut source);
            if let Err(err) = foo {
                error!("Error: {}", err);
                None
            } else {
                foo.ok()
            }
        })
        // Handle the indexing of a `Feed` into the Database.
        .for_each(|feed| {
            if let Err(err) = feed.index() {
                error!("Error While trying to update the database.");
                error!("Error msg: {}", err);
            }
        });

    info!("Indexing done.");
}

/// Retrieves all `Sources` from the database and updates/indexes them.
pub fn index_all() -> Result<()> {
    let sources = dbqueries::get_sources()?;
    index_loop(sources);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::BufReader;
    use database::truncate_db;

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

        index_all().unwrap();

        // Run again to cover Unique constrains erros.
        index_all().unwrap();
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
