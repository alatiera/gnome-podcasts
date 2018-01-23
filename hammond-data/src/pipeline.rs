// FIXME:
//! Docs.

use futures::future::*;
use futures_cpupool::CpuPool;
// use futures::prelude::*;

use hyper::Client;
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
pub fn pipeline<S: IntoIterator<Item = Source>>(sources: S, ignore_etags: bool) -> Result<()> {
    let _pool = CpuPool::new_num_cpus();

    let mut core = Core::new()?;
    let handle = core.handle();
    let client = Client::configure()
        // FIXME: numcpus instead of 4
        .connector(HttpsConnector::new(4, &handle)?)
        .build(&handle);

    let list = sources
        .into_iter()
        .map(|s| s.into_feed(&client, ignore_etags))
        .map(|fut| fut.and_then(clone!(_pool => move |feed| _pool.clone().spawn(feed.index()))))
        .collect();

    let f = core.run(collect_futures(list))?;
    f.into_iter()
        .filter_map(|x| x.err())
        .for_each(|err| error!("Error: {}", err));

    Ok(())
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
