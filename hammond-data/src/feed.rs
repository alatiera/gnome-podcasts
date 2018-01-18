//! Index Feeds.

use futures::future::*;
// use futures::prelude::*;
use errors::*;
use models::{NewEpisode, NewPodcast, Podcast};
use rayon::prelude::*;
use rss;
// use models::{IndexState, Source};
// use pipeline::*;

#[derive(Debug)]
/// Wrapper struct that hold a `Source` id and the `rss::Channel`
/// that corresponds to the `Source.uri` field.
pub struct Feed {
    channel: rss::Channel,
    source_id: i32,
}

impl Feed {
    /// Constructor that consumes a `Source` and a `rss::Channel` returns a `Feed` struct.
    pub fn from_channel_source(channel: rss::Channel, source_id: i32) -> Feed {
        Feed { channel, source_id }
    }

    /// Index the contents of the RSS `Feed` into the database.
    pub fn index(&self) -> Result<()> {
        let pd = self.parse_podcast().into_podcast()?;
        self.index_channel_items(&pd)
    }

    // TODO: Refactor transcactions and find a way to do it in parallel.
    fn index_channel_items(&self, pd: &Podcast) -> Result<()> {
        let episodes = self.parse_channel_items(pd);

        episodes.iter().for_each(|x| {
            if let Err(err) = x.index() {
                error!("Failed to index episode: {:?}.", x.title());
                error!("Error msg: {}", err);
            };
        });
        Ok(())
    }

    fn parse_podcast(&self) -> NewPodcast {
        NewPodcast::new(&self.channel, self.source_id)
    }

    #[allow(dead_code)]
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
}
//     fn parse_channel_items2(
//         &self,
//         pd: &Podcast,
//     ) -> (Vec<IndexState<NewEpisode>>, Vec<IndexState<NewEpisode>>) {
//         let items = self.channel.items();
//         let (insert, update): (Vec<_>, Vec<_>) = items
//             .into_iter()
//             .filter_map(|item| glue(item, pd.id()).ok())
//             .filter(|&state| state == IndexState::NotChanged)
//             .partition(|&state| state == IndexState::Index);

//         (insert, update)
//     }
// }

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
