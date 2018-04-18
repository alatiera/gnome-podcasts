// FIXME:
//! Docs.

#![allow(unused)]

use futures::future::*;
use futures::prelude::*;
use futures::stream::*;

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

type HttpsClient = Client<HttpsConnector<HttpConnector>>;

/// The pipline to be run for indexing and updating a Podcast feed that originates from
/// `Source.uri`.
///
/// Messy temp diagram:
/// Source -> GET Request -> Update Etags -> Check Status -> Parse `xml/Rss` ->
/// Convert `rss::Channel` into `Feed` -> Index Podcast -> Index Episodes.
pub fn pipeline<'a, S>(
    sources: S,
    ignore_etags: bool,
    client: HttpsClient,
) -> Box<Future<Item = Vec<()>, Error = DataError> + 'a>
where
    S: IntoIterator<Item = Source> + 'a,
{
    let stream = iter_ok::<_, DataError>(sources);
    let pipeline = stream
        .and_then(clone!(client => move |s| s.into_feed(client.clone(), ignore_etags)))
        .and_then(|feed| feed.index())
        // the stream will stop at the first error so
        // we ensure that everything will succeded regardless.
        .map_err(|err| error!("Error: {}", err))
        .then(|_| ok::<(), DataError>(()))
        .collect();

    Box::new(pipeline)
}

/// Creates a tokio `reactor::Core`, and a `hyper::Client` and
/// runs the pipeline.
pub fn run<S>(sources: S, ignore_etags: bool) -> Result<(), DataError>
where
    S: IntoIterator<Item = Source>,
{
    let mut core = Core::new()?;
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(num_cpus::get(), &handle)?)
        .build(&handle);

    let p = pipeline(sources, ignore_etags, client);
    core.run(p).map(|_| ())
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

pub(crate) fn glue(item: &rss::Item, id: i32) -> Result<IndexState<NewEpisode>, DataError> {
    NewEpisodeMinimal::new(item, id).and_then(move |ep| determine_ep_state(ep, item))
}

#[cfg(test)]
mod tests {
    use super::*;
    use database::truncate_db;
    use Source;

    // (path, url) tuples.
    const URLS: &[&str] = &[
        "https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.\
         com/InterceptedWithJeremyScahill",
        "https://web.archive.org/web/20180120110314if_/https://feeds.feedburner.com/linuxunplugged",
        "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff",
        "https://web.archive.org/web/20180120104957if_/https://rss.art19.com/steal-the-stars",
        "https://web.archive.org/web/20180120104741if_/https://www.greaterthancode.\
         com/feed/podcast",
    ];

    #[test]
    /// Insert feeds and update/index them.
    fn test_pipeline() {
        truncate_db().unwrap();
        let bad_url = "https://gitlab.gnome.org/World/hammond.atom";
        // if a stream returns error/None it stops
        // bad we want to parse all feeds regardless if one fails
        Source::from_url(bad_url).unwrap();

        URLS.iter().for_each(|url| {
            // Index the urls into the source table.
            Source::from_url(url).unwrap();
        });

        let sources = dbqueries::get_sources().unwrap();
        run(sources, true).unwrap();

        let sources = dbqueries::get_sources().unwrap();
        // Run again to cover Unique constrains erros.
        run(sources, true).unwrap();

        // Assert the index rows equal the controlled results
        assert_eq!(dbqueries::get_sources().unwrap().len(), 6);
        assert_eq!(dbqueries::get_podcasts().unwrap().len(), 5);
        assert_eq!(dbqueries::get_episodes().unwrap().len(), 354);
    }
}
