//! Index Feeds.

use rayon::prelude::*;
use diesel::prelude::*;
use rayon::iter::IntoParallelIterator;

use diesel::associations::Identifiable;
use rss;

use dbqueries;
use parser;

use models::queryables::{Episode, Podcast, Source};
use models::insertables::{NewEpisode, NewPodcast};
use database::connection;
use errors::*;

#[derive(Debug)]
/// Wrapper struct that hold a `Source` and the `rss::Channel`
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
    pub fn from_channel_source(chan: rss::Channel, s: i32) -> Feed {
        Feed {
            channel: chan,
            source_id: s,
        }
    }

    fn index(&self) -> Result<()> {
        let pd = self.get_podcast()?;
        self.index_channel_items(&pd)
    }

    // #[allow(dead_code)]
    // fn index_channel(&self) -> Result<()> {
    //     self.parse_channel().index()?;
    //     Ok(())
    // }

    // TODO: Refactor transcactions and find a way to do it in parallel.
    fn index_channel_items(&self, pd: &Podcast) -> Result<()> {
        let episodes = self.parse_channel_items(pd);
        let db = connection();
        let con = db.get()?;

        let _ = con.transaction::<(), Error, _>(|| {
            episodes.into_iter().for_each(|x| {
                let e = x.index(&con);
                if let Err(err) = e {
                    error!("Failed to index episode: {:?}.", x.title());
                    error!("Error msg: {}", err);
                };
            });
            Ok(())
        });
        Ok(())
    }

    fn parse_channel(&self) -> NewPodcast {
        parser::new_podcast(&self.channel, self.source_id)
    }

    fn parse_channel_items(&self, pd: &Podcast) -> Vec<NewEpisode> {
        let items = self.channel.items();
        let new_episodes: Vec<_> = items
            .par_iter()
            .filter_map(|item| parser::new_episode(item, *pd.id()).ok())
            .collect();

        new_episodes
    }

    fn get_podcast(&self) -> Result<Podcast> {
        self.parse_channel().into_podcast()
    }

    #[allow(dead_code)]
    fn get_episodes(&self) -> Result<Vec<Episode>> {
        let pd = self.get_podcast()?;
        let eps = self.parse_channel_items(&pd);

        let db = connection();
        let con = db.get()?;
        // TODO: Make it parallel
        // This returns only the episodes in the xml feed.
        let episodes: Vec<_> = eps.into_iter()
            .filter_map(|ep| ep.into_episode(&con).ok())
            .collect();

        Ok(episodes)
    }
}

/// Handle the indexing of a `Feed` into the Database.
pub fn index(feed: &Feed) {
    if let Err(err) = feed.index() {
        error!("Error While trying to update the database.");
        error!("Error msg: {}", err);
    };
}

/// Consume a `Source` and return a `Feed`.
fn fetch(source: &mut Source) -> Result<Feed> {
    Feed::from_source(source)
}

/// Index a "list" of `Source`s.
pub fn index_loop<S: IntoParallelIterator<Item = Source>>(sources: S) {
    sources
        .into_par_iter()
        .filter_map(|mut x| {
            let foo = fetch(&mut x);
            if let Err(err) = foo {
                error!("Error: {}", err);
                None
            } else {
                foo.ok()
            }
        })
        .for_each(|x| index(&x));

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
                Feed::from_channel_source(chan, *s.id())
            })
            .collect();

        // Index the channels
        feeds.par_iter().for_each(|x| index(&x));

        // Assert the index rows equal the controlled results
        assert_eq!(dbqueries::get_sources().unwrap().len(), 4);
        assert_eq!(dbqueries::get_podcasts().unwrap().len(), 4);
        assert_eq!(dbqueries::get_episodes().unwrap().len(), 274);
    }

    #[test]
    fn test_partial_index_podcast() {
        truncate_db().unwrap();
        let url = "https://feeds.feedburner.com/InterceptedWithJeremyScahill";

        let mut s1 = Source::from_url(url).unwrap();
        let mut s2 = Source::from_url(url).unwrap();
        assert_eq!(s1, s2);
        assert_eq!(s1.id(), s2.id());

        let f1 = s1.into_feed(false).unwrap();
        let f2 = s2.into_feed(false).unwrap();

        let p1 = f1.get_podcast().unwrap();
        let p2 = {
            f2.index().unwrap();
            f2.get_podcast().unwrap()
        };
        assert_eq!(p1, p2);
        assert_eq!(p1.id(), p2.id());
        assert_eq!(p1.source_id(), p2.source_id());

        let eps1 = f1.get_episodes().unwrap();
        let eps2 = {
            f2.index().unwrap();
            f2.get_episodes().unwrap()
        };

        eps1.into_par_iter().zip(eps2).into_par_iter().for_each(
            |(ep1, ep2): (Episode, Episode)| {
                assert_eq!(ep1, ep2);
                assert_eq!(ep1.id(), ep2.id());
                assert_eq!(ep1.podcast_id(), ep2.podcast_id());
            },
        );
    }
}
