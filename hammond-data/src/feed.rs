use rayon::prelude::*;
use diesel::Identifiable;
use diesel::prelude::*;

use rss;

use dbqueries;
use parser;
use connection;

use models::{Podcast, Source};
use errors::*;


#[derive(Debug)]
pub struct Feed {
    channel: rss::Channel,
    source: Source,
}

impl Feed {
    pub fn from_source(s: Source) -> Result<Feed> {
        s.refresh()
    }

    pub fn from_channel_source(chan: rss::Channel, s: Source) -> Feed {
        Feed {
            channel: chan,
            source: s,
        }
    }

    fn index(&self) -> Result<()> {
        let pd = self.index_channel()?;

        self.index_channel_items(&pd)?;
        Ok(())
    }

    fn index_channel(&self) -> Result<Podcast> {
        let pd = parser::new_podcast(&self.channel, *self.source.id());
        // Convert NewPodcast to Podcast
        pd.into_podcast()
    }

    // TODO: Refactor transcactions and find a way to do it in parallel.
    fn index_channel_items(&self, pd: &Podcast) -> Result<()> {
        let items = self.channel.items();
        let episodes: Vec<_> = items
            .into_par_iter()
            .map(|item| parser::new_episode(item, *pd.id()))
            .collect();

        let tempdb = connection().get().unwrap();
        let _ = tempdb.transaction::<(), Error, _>(|| {
            episodes.into_iter().for_each(|x| {
                let e = x.index(&*tempdb);
                if let Err(err) = e {
                    error!("Failed to index episode: {:?}.", x);
                    error!("Error msg: {}", err);
                };
            });
            Ok(())
        });
        Ok(())
    }
}

pub fn index_all() -> Result<()> {
    let mut f = fetch_all()?;

    index(&mut f);
    info!("Indexing done.");
    Ok(())
}

pub fn index(feeds: &mut [Feed]) {
    feeds.into_par_iter().for_each(|f| {
        let e = f.index();
        if e.is_err() {
            error!("Error While trying to update the database.");
            error!("Error msg: {}", e.unwrap_err());
        };
    });
}

pub fn fetch_all() -> Result<Vec<Feed>> {
    let feeds = dbqueries::get_sources()?;

    let results = fetch(feeds);
    Ok(results)
}

pub fn fetch(feeds: Vec<Source>) -> Vec<Feed> {
    let results: Vec<_> = feeds
        .into_par_iter()
        .filter_map(|x| {
            let uri = x.uri().to_owned();
            let l = Feed::from_source(x);
            if l.is_ok() {
                l.ok()
            } else {
                error!("Error While trying to fetch from source: {}.", uri);
                error!("Error msg: {}", l.unwrap_err());
                None
            }
        })
        .collect();

    results
}

#[cfg(test)]
mod tests {

    use rss;
    use models::NewSource;

    use std::fs;
    use std::io::BufReader;

    use super::*;

    #[test]
    /// Insert feeds and update/index them.
    fn test_index_loop() {
        let inpt = vec![
            "https://request-for-explanation.github.io/podcast/rss.xml",
            "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
            "http://feeds.propublica.org/propublica/podcast",
            "http://feeds.feedburner.com/linuxunplugged",
        ];

        inpt.iter().for_each(|feed| {
            NewSource::new_with_uri(feed).into_source().unwrap();
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

        let mut feeds: Vec<_> = urls.iter()
            .map(|&(path, url)| {
                // Create and insert a Source into db
                let s = NewSource::new_with_uri(url).into_source().unwrap();

                // open the xml file
                let feed = fs::File::open(path).unwrap();
                // parse it into a channel
                let chan = rss::Channel::read_from(BufReader::new(feed)).unwrap();
                Feed::from_channel_source(chan, s)
            })
            .collect();

        // Index the channels
        index(&mut feeds);

        // Assert the index rows equal the controlled results
        assert_eq!(dbqueries::get_sources().unwrap().len(), 4);
        assert_eq!(dbqueries::get_podcasts().unwrap().len(), 4);
        assert_eq!(dbqueries::get_episodes().unwrap().len(), 274);
    }
}
