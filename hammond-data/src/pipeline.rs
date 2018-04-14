// FIXME:
//! Docs.

use futures::future::*;
// use futures::prelude::*;

use hyper::client::HttpConnector;
use hyper::Client;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;

use num_cpus;
use rss;

use dbqueries;
use errors::DataError;
use models::{IndexState, NewEpisode, NewEpisodeMinimal};
use Source;

// use std::sync::{Arc, Mutex};

// http://gtk-rs.org/tuto/closures
#[macro_export]
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
pub fn pipeline<S: IntoIterator<Item = Source>>(
    sources: S,
    ignore_etags: bool,
    tokio_core: &mut Core,
    client: Client<HttpsConnector<HttpConnector>>,
) -> Result<(), DataError> {
    let list: Vec<_> = sources
        .into_iter()
        .map(clone!(client => move |s| s.into_feed(client.clone(), ignore_etags)))
        .map(|fut| fut.and_then(|feed| feed.index()))
        .map(|fut| fut.map(|_| ()).map_err(|err| error!("Error: {}", err)))
        .collect();

    if list.is_empty() {
        return Err(DataError::EmptyFuturesList);
    }

    // Thats not really concurrent yet I think.
    tokio_core.run(collect_futures(list))?;

    Ok(())
}

/// Creates a tokio `reactor::Core`, and a `hyper::Client` and
/// runs the pipeline.
pub fn run(sources: Vec<Source>, ignore_etags: bool) -> Result<(), DataError> {
    if sources.is_empty() {
        return Ok(());
    }

    let mut core = Core::new()?;
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(num_cpus::get(), &handle)?)
        .build(&handle);

    pipeline(sources, ignore_etags, &mut core, client)
}

/// Docs
pub fn index_single_source(s: Source, ignore_etags: bool) -> Result<(), DataError> {
    let mut core = Core::new()?;
    let handle = core.handle();

    let client = Client::configure()
        .connector(HttpsConnector::new(num_cpus::get(), &handle)?)
        .build(&handle);

    let work = s.into_feed(client, ignore_etags)
        .and_then(move |feed| feed.index())
        .map(|_| ());

    core.run(work)
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

pub(crate) fn glue_async<'a>(
    item: &'a rss::Item,
    id: i32,
) -> Box<Future<Item = IndexState<NewEpisode>, Error = DataError> + 'a> {
    Box::new(
        result(NewEpisodeMinimal::new(item, id)).and_then(move |ep| determine_ep_state(ep, item)),
    )
}

// Weird magic from #rust irc channel
// kudos to remexre
/// FIXME: Docs
#[cfg_attr(feature = "cargo-clippy", allow(type_complexity))]
pub fn collect_futures<F>(
    futures: Vec<F>,
) -> Box<Future<Item = Vec<Result<F::Item, F::Error>>, Error = DataError>>
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
    use database::truncate_db;
    use Source;

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
