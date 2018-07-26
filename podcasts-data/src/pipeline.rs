// FIXME:
//! Docs.

use futures::future::*;
use futures::prelude::*;
use futures::stream::*;

use hyper::client::HttpConnector;
use hyper::Client;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;

use num_cpus;
use rayon;
use rayon_futures::ScopeFutureExt;

use errors::DataError;
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
    client: &HttpsClient,
) -> impl Future<Item = Vec<()>, Error = DataError> + 'a
where
    S: Stream<Item = Source, Error = DataError> + 'a,
{
    sources
        .and_then(clone!(client => move |s| s.into_feed(client.clone(), ignore_etags)))
        .and_then(|feed| rayon::scope(|s| s.spawn_future(feed.index())))
        // the stream will stop at the first error so
        // we ensure that everything will succeded regardless.
        .map_err(|err| error!("Error: {}", err))
        .then(|_| ok::<(), DataError>(()))
        .collect()
}

/// Creates a tokio `reactor::Core`, and a `hyper::Client` and
/// runs the pipeline to completion. The `reactor::Core` is dropped afterwards.
pub fn run<S>(sources: S, ignore_etags: bool) -> Result<(), DataError>
where
    S: IntoIterator<Item = Source>,
{
    let mut core = Core::new()?;
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(num_cpus::get(), &handle)?)
        .build(&handle);

    let stream = iter_ok::<_, DataError>(sources);
    let p = pipeline(stream, ignore_etags, &client);
    core.run(p).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use database::truncate_db;
    use dbqueries;
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
        let bad_url = "https://gitlab.gnome.org/World/podcasts.atom";
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
