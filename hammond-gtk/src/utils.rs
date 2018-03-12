#![cfg_attr(feature = "cargo-clippy", allow(type_complexity))]

use failure::Error;
use gdk_pixbuf::Pixbuf;
use regex::Regex;
use reqwest;
use send_cell::SendCell;
use serde_json::Value;

// use hammond_data::feed;
use hammond_data::{PodcastCoverQuery, Source};
use hammond_data::dbqueries;
use hammond_data::pipeline;
use hammond_downloader::downloader;

use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use std::sync::mpsc::Sender;
use std::thread;

use app::Action;

pub fn refresh_feed_wrapper(source: Option<Vec<Source>>, sender: Sender<Action>) {
    if let Err(err) = refresh_feed(source, sender) {
        error!("An error occured while trying to update the feeds.");
        error!("Error: {}", err);
    }
}

/// Update the rss feed(s) originating from `source`.
/// If `source` is None, Fetches all the `Source` entries in the database and updates them.
/// When It's done,it queues up a `RefreshViews` action.
fn refresh_feed(source: Option<Vec<Source>>, sender: Sender<Action>) -> Result<(), Error> {
    sender.send(Action::HeaderBarShowUpdateIndicator)?;

    thread::spawn(move || {
        let mut sources = source.unwrap_or_else(|| {
            dbqueries::get_sources().expect("Failed to retrieve Sources from the database.")
        });

        // Work around to improve the feed addition experience.
        // Many times links to rss feeds are just redirects(usually to an https
        // version). Sadly I haven't figured yet a nice way to follow up links
        // redirects without getting to lifetime hell with futures and hyper.
        // So the requested refresh is only of 1 feed, and the feed fails to be indexed,
        // (as a 301 redict would update the source entry and exit), another refresh is
        // run. For more see hammond_data/src/models/source.rs `fn
        // request_constructor`. also ping me on irc if or open an issue if you
        // want to tackle it.
        if sources.len() == 1 {
            let source = sources.remove(0);
            let id = source.id();
            if let Err(err) = pipeline::index_single_source(source, false) {
                error!("Error While trying to update the database.");
                error!("Error msg: {}", err);
                if let Ok(source) = dbqueries::get_source_from_id(id) {
                    if let Err(err) = pipeline::index_single_source(source, false) {
                        error!("Error While trying to update the database.");
                        error!("Error msg: {}", err);
                    }
                }
            }
        // This is what would normally run
        } else if let Err(err) = pipeline::run(sources, false) {
            error!("Error While trying to update the database.");
            error!("Error msg: {}", err);
        }

        sender
            .send(Action::HeaderBarHideUpdateIndicator)
            .expect("Action channel blew up.");
        sender
            .send(Action::RefreshAllViews)
            .expect("Action channel blew up.");
    });
    Ok(())
}

lazy_static! {
    static ref CACHED_PIXBUFS: RwLock<HashMap<(i32, u32), Mutex<SendCell<Pixbuf>>>> =
        { RwLock::new(HashMap::new()) };
}

// Since gdk_pixbuf::Pixbuf is refference counted and every episode,
// use the cover of the Podcast Feed/Show, We can only create a Pixbuf
// cover per show and pass around the Rc pointer.
//
// GObjects do not implement Send trait, so SendCell is a way around that.
// Also lazy_static requires Sync trait, so that's what the mutexes are.
// TODO: maybe use something that would just scale to requested size?
pub fn get_pixbuf_from_path(pd: &PodcastCoverQuery, size: u32) -> Result<Pixbuf, Error> {
    {
        let hashmap = CACHED_PIXBUFS
            .read()
            .map_err(|_| format_err!("Failed to get a lock on the pixbuf cache mutex."))?;
        if let Some(px) = hashmap.get(&(pd.id(), size)) {
            let m = px.lock()
                .map_err(|_| format_err!("Failed to lock pixbuf mutex."))?;
            return Ok(m.clone().into_inner());
        }
    }

    let img_path = downloader::cache_image(pd)?;
    let px = Pixbuf::new_from_file_at_scale(&img_path, size as i32, size as i32, true)?;
    let mut hashmap = CACHED_PIXBUFS
        .write()
        .map_err(|_| format_err!("Failed to lock pixbuf mutex."))?;
    hashmap.insert((pd.id(), size), Mutex::new(SendCell::new(px.clone())));
    Ok(px)
}

#[inline]
// FIXME: the signature should be `fn foo(s: Url) -> Result<Url, Error>`
pub fn itunes_to_rss(url: &str) -> Result<String, Error> {
    let id = itunes_id_from_url(url).ok_or_else(|| format_err!("Failed to find an Itunes ID."))?;
    lookup_id(id)
}

#[inline]
fn itunes_id_from_url(url: &str) -> Option<u32> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"/id([0-9]+)").unwrap();
    }

    // Get the itunes id from the url
    let foo = RE.captures_iter(url).nth(0)?.get(1)?.as_str();
    // Parse it to a u32, this *should* never fail
    foo.parse::<u32>().ok()
}

#[inline]
fn lookup_id(id: u32) -> Result<String, Error> {
    let url = format!("https://itunes.apple.com/lookup?id={}&entity=podcast", id);
    let req: Value = reqwest::get(&url)?.json()?;
    // FIXME: First time using serde, this could be done better and avoid using [] for indexing.
    let feedurl = req["results"][0]["feedUrl"].as_str();
    let feedurl = feedurl.ok_or_else(|| format_err!("Failed to get url from itunes response"))?;
    Ok(feedurl.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hammond_data::Source;
    use hammond_data::dbqueries;

    #[test]
    // This test inserts an rss feed to your `XDG_DATA/hammond/hammond.db` so we make it explicit
    // to run it.
    #[ignore]
    fn test_get_pixbuf_from_path() {
        let url = "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff";
        // Create and index a source
        let source = Source::from_url(url).unwrap();
        // Copy it's id
        let sid = source.id();
        pipeline::run(vec![source], true).unwrap();

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid).unwrap();
        let pxbuf = get_pixbuf_from_path(&pd.into(), 256);
        assert!(pxbuf.is_ok());
    }

    #[test]
    fn test_itunes_to_rss() {
        let itunes_url = "https://itunes.apple.com/podcast/id1195206601";
        let rss_url = String::from("http://feeds.feedburner.com/InterceptedWithJeremyScahill");
        assert_eq!(rss_url, itunes_to_rss(itunes_url).unwrap());
    }

    #[test]
    fn test_itunes_id() {
        let id = 1195206601;
        let itunes_url = "https://itunes.apple.com/podcast/id1195206601";
        assert_eq!(id, itunes_id_from_url(itunes_url).unwrap());
    }

    #[test]
    fn test_itunes_lookup_id() {
        let id = 1195206601;
        let rss_url = "http://feeds.feedburner.com/InterceptedWithJeremyScahill";
        assert_eq!(rss_url, lookup_id(id).unwrap());
    }
}
