// FIXME:
//! Docs.

use futures::future::*;
use futures_cpupool::CpuPool;
// use futures::prelude::*;

use hyper::Client;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;

use rss;

use Source;
use dbqueries;
use errors::*;
use models::{IndexState, NewEpisode, NewEpisodeMinimal};
// use Feed;

use std;
// use std::sync::{Arc, Mutex};

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

/// The pipline to be run for indexing and updating a Podcast feed that originates from
/// `Source.uri`.
///
/// Messy temp diagram:
/// Source -> GET Request -> Update Etags -> Check Status -> Parse xml/Rss ->
/// Convert `rss::Channel` into Feed -> Index Podcast -> Index Episodes.
///
/// # Panics
/// If `sources` contains no Items.
pub fn pipeline<S: IntoIterator<Item = Source>>(
    sources: S,
    ignore_etags: bool,
    tokio_core: &mut Core,
    pool: CpuPool,
    client: Client<HttpsConnector<HttpConnector>>,
) -> Result<()> {
    let list: Vec<_> = sources
        .into_iter()
        .map(clone!(pool => move |s| s.into_feed(&client, pool.clone(), ignore_etags)))
        .map(|fut| fut.and_then(clone!(pool => move |feed| pool.clone().spawn(feed.index()))))
        .map(|fut| fut.map(|_| ()).map_err(|err| error!("Error: {}", err)))
        .collect();

    assert!(!list.is_empty());
    // Thats not really concurrent yet I think.
    tokio_core.run(collect_futures(list))?;

    Ok(())
}

/// Creates a tokio-core, a  cpu_pool, and a hyper::Client and runs the pipeline.
pub fn run<S: IntoIterator<Item = Source>>(sources: S, ignore_etags: bool) -> Result<()> {
    let pool = CpuPool::new_num_cpus();
    let mut core = Core::new()?;
    let handle = core.handle();
    let client = Client::configure()
        // FIXME: numcpus instead of 4
        .connector(HttpsConnector::new(4, &handle)?)
        .build(&handle);

    pipeline(sources, ignore_etags, &mut core, pool, client)
}

fn determine_ep_state(ep: NewEpisodeMinimal, item: &rss::Item) -> Result<IndexState<NewEpisode>> {
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

pub(crate) fn glue_async<'a>(
    item: &'a rss::Item,
    id: i32,
) -> Box<Future<Item = IndexState<NewEpisode>, Error = Error> + 'a> {
    Box::new(
        result(NewEpisodeMinimal::new(item, id)).and_then(move |ep| determine_ep_state(ep, item)),
    )
}

// Weird magic from #rust irc channel
// kudos to remexre
/// docs
pub fn collect_futures<F>(
    futures: Vec<F>,
) -> Box<Future<Item = Vec<std::result::Result<F::Item, F::Error>>, Error = Error>>
where
    F: 'static + Future,
    <F as Future>::Item: 'static,
    <F as Future>::Error: 'static,
{
    Box::new(loop_fn((futures, vec![]), |(futures, mut done)| {
        select_all(futures).then(|r| {
            let (r, rest) = match r {
                Ok((r, _, rest)) => (Ok(r), rest),
                Err((r, _, rest)) => (Err(r), rest),
            };
            done.push(r);
            if rest.is_empty() {
                Ok(Loop::Break(done))
            } else {
                Ok(Loop::Continue((rest, done)))
            }
        })
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use Source;
    use database::truncate_db;

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
    /// Insert feeds and update/index them.
    fn test_pipeline() {
        truncate_db().unwrap();
        URLS.iter().for_each(|&(_, url)| {
            // Index the urls into the source table.
            Source::from_url(url).unwrap();
        });
        let sources = dbqueries::get_sources().unwrap();
        run(sources, true).unwrap();

        let sources = dbqueries::get_sources().unwrap();
        // Run again to cover Unique constrains erros.
        run(sources, true).unwrap();

        // Assert the index rows equal the controlled results
        assert_eq!(dbqueries::get_sources().unwrap().len(), 5);
        assert_eq!(dbqueries::get_podcasts().unwrap().len(), 5);
        assert_eq!(dbqueries::get_episodes().unwrap().len(), 354);
    }
}
