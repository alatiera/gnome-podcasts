use gdk_pixbuf::Pixbuf;
use send_cell::SendCell;

// use hammond_data::feed;
use hammond_data::{PodcastCoverQuery, Source};
use hammond_data::dbqueries;
use hammond_data::pipeline;
use hammond_downloader::downloader;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::Sender;
use std::thread;

use app::Action;
use headerbar::Header;

/// Update the rss feed(s) originating from `source`.
/// If `source` is None, Fetches all the `Source` entries in the database and updates them.
/// When It's done,it queues up a `RefreshViews` action.
pub fn refresh_feed(headerbar: Arc<Header>, source: Option<Vec<Source>>, sender: Sender<Action>) {
    // TODO: make it an application channel action.
    // I missed it before apparently.
    headerbar.show_update_notification();

    thread::spawn(move || {
        // FIXME: This is messy at best.
        if let Some(s) = source {
            // feed::index_loop(s);
            // TODO: determine if it needs to ignore_etags.
            if let Err(err) = pipeline::run(s, true) {
                error!("Error While trying to update the database.");
                error!("Error msg: {}", err);
            }
        } else {
            let sources = dbqueries::get_sources().unwrap();
            if let Err(err) = pipeline::run(sources, false) {
                error!("Error While trying to update the database.");
                error!("Error msg: {}", err);
            }
        };

        sender.send(Action::HeaderBarHideUpdateIndicator).unwrap();
        sender.send(Action::RefreshAllViews).unwrap();
    });
}

lazy_static! {
    static ref CACHED_PIXBUFS: RwLock<HashMap<(i32, u32), Mutex<SendCell<Pixbuf>>>> = {
        RwLock::new(HashMap::new())
    };
}

// Since gdk_pixbuf::Pixbuf is refference counted and every episode,
// use the cover of the Podcast Feed/Show, We can only create a Pixbuf
// cover per show and pass around the Rc pointer.
//
// GObjects do not implement Send trait, so SendCell is a way around that.
// Also lazy_static requires Sync trait, so that's what the mutexes are.
// TODO: maybe use something that would just scale to requested size?
pub fn get_pixbuf_from_path(pd: &PodcastCoverQuery, size: u32) -> Option<Pixbuf> {
    {
        let hashmap = CACHED_PIXBUFS.read().unwrap();
        let res = hashmap.get(&(pd.id(), size));
        if let Some(px) = res {
            let m = px.lock().unwrap();
            return Some(m.clone().into_inner());
        }
    }

    let img_path = downloader::cache_image(pd)?;
    let px = Pixbuf::new_from_file_at_scale(&img_path, size as i32, size as i32, true).ok();
    if let Some(px) = px {
        let mut hashmap = CACHED_PIXBUFS.write().unwrap();
        hashmap.insert((pd.id(), size), Mutex::new(SendCell::new(px.clone())));
        return Some(px);
    }
    None
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
        assert!(pxbuf.is_some());
    }
}
