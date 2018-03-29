#![cfg_attr(feature = "cargo-clippy", allow(type_complexity))]

use gdk_pixbuf::Pixbuf;
use gio::{Settings, SettingsExt};
use glib;
use gtk;
use gtk::prelude::*;

use failure::Error;
use rayon;
use regex::Regex;
use reqwest;
use send_cell::SendCell;
use serde_json::Value;

// use hammond_data::feed;
use hammond_data::{PodcastCoverQuery, Source};
use hammond_data::dbqueries;
use hammond_data::pipeline;
use hammond_data::utils::checkup;
use hammond_downloader::downloader;

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock};
use std::sync::Arc;
use std::sync::mpsc::*;
use std::thread;

use app::Action;

use chrono::Duration;
use chrono::prelude::*;

lazy_static! {
    static ref IGNORESHOWS: Arc<Mutex<HashSet<i32>>> = Arc::new(Mutex::new(HashSet::new()));
}

pub fn ignore_show(id: i32) -> Result<(), Error> {
    let mut guard = IGNORESHOWS.lock().map_err(|err| format_err!("{}", err))?;
    guard.insert(id);
    Ok(())
}

pub fn uningore_show(id: i32) -> Result<(), Error> {
    let mut guard = IGNORESHOWS.lock().map_err(|err| format_err!("{}", err))?;
    guard.remove(&id);
    Ok(())
}

pub fn get_ignored_shows() -> Result<Vec<i32>, Error> {
    let guard = IGNORESHOWS.lock().map_err(|err| format_err!("{}", err))?;
    let keys = guard.iter().cloned().collect::<Vec<_>>();
    Ok(keys)
}

pub fn cleanup(cleanup_date: DateTime<Utc>) {
    if let Err(err) = checkup(cleanup_date) {
        error!("Check up failed: {}", err);
    }
}

pub fn refresh(source: Option<Vec<Source>>, sender: Sender<Action>) {
    if let Err(err) = refresh_feed(source, sender) {
        error!("An error occured while trying to update the feeds.");
        error!("Error: {}", err);
    }
}

pub fn get_refresh_interval(settings: &Settings) -> Duration {
    let time = settings.get_int("refresh-interval-time") as i64;
    let period = settings.get_string("refresh-interval-period").unwrap();

    time_period_to_duration(time, period.as_str())
}

pub fn get_cleanup_date(settings: &Settings) -> DateTime<Utc> {
    let time = settings.get_int("cleanup-age-time") as i64;
    let period = settings.get_string("cleanup-age-period").unwrap();
    let duration = time_period_to_duration(time, period.as_str());

    Utc::now() - duration
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
    static ref COVER_DL_REGISTRY: RwLock<HashSet<i32>> = RwLock::new(HashSet::new());
    static ref THREADPOOL: rayon::ThreadPool = rayon::ThreadPoolBuilder::new()
        .breadth_first()
        .build()
        .unwrap();
}

// Since gdk_pixbuf::Pixbuf is refference counted and every episode,
// use the cover of the Podcast Feed/Show, We can only create a Pixbuf
// cover per show and pass around the Rc pointer.
//
// GObjects do not implement Send trait, so SendCell is a way around that.
// Also lazy_static requires Sync trait, so that's what the mutexes are.
// TODO: maybe use something that would just scale to requested size?
pub fn set_image_from_path(
    image: &gtk::Image,
    pd: Arc<PodcastCoverQuery>,
    size: u32,
) -> Result<(), Error> {
    {
        // Check if there's an active download about this show cover.
        // If there is, a callback will be set so this function will be called again.
        // If the download succedes, there should be a quick return from the pixbuf cache_image
        // If it fails another download will be scheduled.
        let reg_guard = COVER_DL_REGISTRY
            .read()
            .map_err(|err| format_err!("Cover Registry: {}.", err))?;
        if reg_guard.contains(&pd.id()) {
            let callback = clone!(image, pd => move || {
                 let _ = set_image_from_path(&image, pd.clone(), size);
                 glib::Continue(false)
            });
            gtk::timeout_add(250, callback);
            return Ok(());
        }
    }

    {
        let hashmap = CACHED_PIXBUFS
            .read()
            .map_err(|err| format_err!("Pixbuf HashMap: {}", err))?;
        if let Some(px) = hashmap.get(&(pd.id(), size)) {
            let m = px.lock()
                .map_err(|err| format_err!("SendCell Mutex: {}", err))?;
            let px = m.try_get().ok_or_else(|| {
                format_err!("Pixbuf was accessed from a different thread than created")
            })?;
            image.set_from_pixbuf(px);
            return Ok(());
        }
    }

    let (sender, receiver) = channel();
    let pd_ = pd.clone();
    THREADPOOL.spawn(move || {
        {
            if let Ok(mut guard) = COVER_DL_REGISTRY.write() {
                guard.insert(pd_.id());
            }
        }
        let _ = sender.send(downloader::cache_image(&pd_));
        {
            if let Ok(mut guard) = COVER_DL_REGISTRY.write() {
                guard.remove(&pd_.id());
            }
        }
    });

    let image = image.clone();
    let s = size as i32;
    gtk::timeout_add(25, move || {
        if let Ok(path) = receiver.try_recv() {
            if let Ok(path) = path {
                if let Ok(px) = Pixbuf::new_from_file_at_scale(&path, s, s, true) {
                    if let Ok(mut hashmap) = CACHED_PIXBUFS.write() {
                        hashmap.insert((pd.id(), size), Mutex::new(SendCell::new(px.clone())));
                        image.set_from_pixbuf(&px);
                    }
                }
            }
            glib::Continue(false)
        } else {
            glib::Continue(true)
        }
    });
    Ok(())
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

pub fn time_period_to_duration(time: i64, period: &str) -> Duration {
    match period {
        "weeks" => Duration::weeks(time),
        "days" => Duration::days(time),
        "hours" => Duration::hours(time),
        "minutes" => Duration::minutes(time),
        _ => Duration::seconds(time),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use hammond_data::Source;
    // use hammond_data::dbqueries;

    #[test]
    fn test_time_period_to_duration() {
        let time = 2;
        let week = 604800 * time;
        let day = 86400 * time;
        let hour = 3600 * time;
        let minute = 60 * time;

        assert_eq!(week, time_period_to_duration(time, "weeks").num_seconds());
        assert_eq!(day, time_period_to_duration(time, "days").num_seconds());
        assert_eq!(hour, time_period_to_duration(time, "hours").num_seconds());
        assert_eq!(
            minute,
            time_period_to_duration(time, "minutes").num_seconds()
        );
        assert_eq!(time, time_period_to_duration(time, "seconds").num_seconds());
    }

    // #[test]
    // This test inserts an rss feed to your `XDG_DATA/hammond/hammond.db` so we make it explicit
    // to run it.
    // #[ignore]
    // Disabled till https://gitlab.gnome.org/alatiera/Hammond/issues/56
    // fn test_set_image_from_path() {
    //     let url = "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff";
    // Create and index a source
    //     let source = Source::from_url(url).unwrap();
    // Copy it's id
    //     let sid = source.id();
    //     pipeline::run(vec![source], true).unwrap();

    // Get the Podcast
    //     let img = gtk::Image::new();
    //     let pd = dbqueries::get_podcast_from_source_id(sid).unwrap().into();
    //     let pxbuf = set_image_from_path(&img, Arc::new(pd), 256);
    //     assert!(pxbuf.is_ok());
    // }

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
