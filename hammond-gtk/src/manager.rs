// use hammond_data::Episode;
use hammond_data::dbqueries;
use hammond_downloader::downloader::get_episode;

use app::Action;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::Sender;
// use std::sync::atomic::AtomicUsize;
// use std::path::PathBuf;
use std::thread;

#[derive(Debug)]
pub struct Progress {
    total_bytes: u64,
    downloaded_bytes: u64,
}

impl Progress {
    pub fn new(size: u64) -> Self {
        Progress {
            total_bytes: size,
            downloaded_bytes: 0,
        }
    }
}

lazy_static! {
    pub static ref ACTIVE_DOWNLOADS: Arc<RwLock<HashSet<i32>>> = {
        Arc::new(RwLock::new(HashSet::new()))
    };

    pub static ref ACTIVE_PROGRESS: Arc<RwLock<HashMap<i32, Mutex<Progress>>>> = {
        Arc::new(RwLock::new(HashMap::new()))
    };
}

pub fn add(id: i32, directory: &str, sender: Sender<Action>) {
    {
        let mut m = ACTIVE_DOWNLOADS.write().unwrap();
        m.insert(id);
    }

    let dir = directory.to_owned();
    thread::spawn(move || {
        let episode = dbqueries::get_episode_from_rowid(id).unwrap();
        let e = get_episode(&mut episode.into(), dir.as_str());
        if let Err(err) = e {
            error!("Error: {}", err);
        };

        {
            let mut m = ACTIVE_DOWNLOADS.write().unwrap();
            m.remove(&id);
        }
        sender.send(Action::RefreshViews).unwrap();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use hammond_downloader::downloader;

    use diesel::Identifiable;

    use hammond_data::database;
    use hammond_data::feed::*;
    use hammond_data::{Episode, Source};
    use hammond_data::dbqueries;

    use std::path::Path;
    use std::{thread, time};
    use std::sync::mpsc::channel;

    #[test]
    // This test inserts an rss feed to your `XDG_DATA/hammond/hammond.db` so we make it explicit
    // to run it.
    #[ignore]
    // THIS IS NOT A RELIABLE TEST
    // Just quick sanity check
    fn test_start_dl() {
        let url = "http://www.newrustacean.com/feed.xml";

        // Create and index a source
        let source = Source::from_url(url).unwrap();
        // Copy it's id
        let sid = source.id().clone();

        // Convert Source it into a Feed and index it
        let feed = source.into_feed(true).unwrap();
        index(&feed);

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid).unwrap();
        // Get an episode
        let episode: Episode = {
            let con = database::connection();
            dbqueries::get_episode_from_pk(&*con.get().unwrap(), "e000: Hello, world!", *pd.id())
                .unwrap()
        };

        let (sender, _rx) = channel();

        let download_fold = downloader::get_download_folder(&pd.title()).unwrap();
        add(episode.rowid(), download_fold.as_str(), sender);

        // Give it soem time to download the file
        thread::sleep(time::Duration::from_secs(40));

        let final_path = format!("{}/{}.unknown", &download_fold, episode.rowid());
        println!("{}", &final_path);
        assert!(Path::new(&final_path).exists());
    }
}
