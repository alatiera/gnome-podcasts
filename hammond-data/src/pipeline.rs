// FIXME:
//! Docs.

use tokio_core::reactor::Core;
use hyper::Client;
use hyper_tls::HttpsConnector;
use futures::prelude::*;
use futures::future::*;

use errors::*;
use Source;
// use Feed;

use std;

/// The pipline to be run for indexing and updating a Podcast feed that originates from
/// `Source.uri`.
///
/// Messy temp diagram:
/// Source -> GET Request -> Update Etags -> Check Status -> Parse xml/Rss ->
/// Convert rss::Channel into Feed -> Index Podcast -> Index Episodes.
pub fn pipeline<S: IntoIterator<Item = Source>>(sources: S, ignore_etags: bool) -> Result<()> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let client = Client::configure()
        // FIXME: numcpus instead of 4
        .connector(HttpsConnector::new(4, &handle)?)
        .build(&handle);

    let list = sources
        .into_iter()
        // FIXME: Make proper indexing futures instead of wrapping up existing
        // blocking functions
        .map(|s| s.into_fututre_feed(&client, ignore_etags).map(|feed| feed.index_future()))
        .collect();

    let f = core.run(collect_futures(list))?;
    f.into_iter()
        .filter_map(|x| x.err())
        .for_each(|err| error!("Error: {}", err));

    Ok(())
}

// Weird magic from #rust irc channel
// kudos to remexre
fn collect_futures<F>(
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
            if rest.len() == 0 {
                Ok(Loop::Break(done))
            } else {
                Ok(Loop::Continue((rest, done)))
            }
        })
    }))
}
