use failure::Error;

// use hammond_data::Episode;
use hammond_data::dbqueries;
use hammond_downloader::downloader::{get_episode, DownloadProgress};

use app::Action;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::Sender;
// use std::sync::atomic::AtomicUsize;
// use std::path::PathBuf;
use std::thread;

// This is messy, undocumented and hacky af.
// I am terrible at writting downloaders and download managers.

#[derive(Debug)]
pub struct Progress {
    total_bytes: u64,
    downloaded_bytes: u64,
    cancel: bool,
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            total_bytes: 0,
            downloaded_bytes: 0,
            cancel: false,
        }
    }
}

impl Progress {
    pub fn get_fraction(&self) -> f64 {
        let ratio = self.downloaded_bytes as f64 / self.total_bytes as f64;
        debug!("{:?}", self);
        debug!("Ratio completed: {}", ratio);

        if ratio >= 1.0 {
            return 1.0;
        };
        ratio
    }

    pub fn get_total_size(&self) -> u64 {
        self.total_bytes
    }

    pub fn get_downloaded(&self) -> u64 {
        self.downloaded_bytes
    }

    pub fn cancel(&mut self) {
        self.cancel = true;
    }
}

impl DownloadProgress for Progress {
    fn set_downloaded(&mut self, downloaded: u64) {
        self.downloaded_bytes = downloaded
    }

    fn set_size(&mut self, bytes: u64) {
        self.total_bytes = bytes;
    }

    fn should_cancel(&self) -> bool {
        self.cancel
    }
}

lazy_static! {
    pub static ref ACTIVE_DOWNLOADS: Arc<RwLock<HashMap<i32, Arc<Mutex<Progress>>>>> =
        { Arc::new(RwLock::new(HashMap::new())) };
}

pub fn add(id: i32, directory: &str, sender: Sender<Action>) -> Result<(), Error> {
    // Create a new `Progress` struct to keep track of dl progress.
    let prog = Arc::new(Mutex::new(Progress::default()));

    {
        let mut m = ACTIVE_DOWNLOADS
            .write()
            .map_err(|_| format_err!("Failed to get a lock on the mutex."))?;
        m.insert(id, prog.clone());
    }

    let dir = directory.to_owned();
    thread::spawn(move || {
        if let Ok(episode) = dbqueries::get_episode_from_rowid(id) {
            let pid = episode.podcast_id();
            let id = episode.rowid();

            if let Err(err) = get_episode(&mut episode.into(), dir.as_str(), Some(prog)) {
                error!("Error while trying to download an episode");
                error!("Error: {}", err);
            }

            {
                if let Ok(mut m) = ACTIVE_DOWNLOADS.write() {
                    info!("Removed: {:?}", m.remove(&id));
                }
            }

            // {
            //     if let Ok(m) = ACTIVE_DOWNLOADS.read() {
            //         debug!("ACTIVE DOWNLOADS: {:#?}", m);
            //     }
            // }

            sender
                .send(Action::RefreshEpisodesView)
                .expect("Action channel blew up.");
            sender
                .send(Action::RefreshWidgetIfSame(pid))
                .expect("Action channel blew up.");
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use hammond_data::{Episode, Source};
    use hammond_data::dbqueries;
    use hammond_data::pipeline;
    use hammond_data::utils::get_download_folder;

    use hammond_downloader::downloader::get_episode;

    use std::{thread, time};
    use std::fs;
    use std::path::Path;
    use std::sync::mpsc::channel;

    #[test]
    // This test inserts an rss feed to your `XDG_DATA/hammond/hammond.db` so we make it explicit
    // to run it.
    #[ignore]
    // THIS IS NOT A RELIABLE TEST
    // Just quick sanity check
    fn test_start_dl() {
        let url = "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff";
        // Create and index a source
        let source = Source::from_url(url).unwrap();
        // Copy it's id
        let sid = source.id();
        pipeline::run(vec![source], true).unwrap();

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid).unwrap();
        let title = "Coming soon... The Tip Off";
        // Get an episode
        let episode: Episode = dbqueries::get_episode_from_pk(title, pd.id()).unwrap();

        let (sender, _rx) = channel();

        let download_fold = get_download_folder(&pd.title()).unwrap();
        add(episode.rowid(), download_fold.as_str(), sender).unwrap();
        assert_eq!(ACTIVE_DOWNLOADS.read().unwrap().len(), 1);

        // Give it soem time to download the file
        thread::sleep(time::Duration::from_secs(20));

        let final_path = format!("{}/{}.mp3", &download_fold, episode.rowid());
        assert!(Path::new(&final_path).exists());
        fs::remove_file(final_path).unwrap();
    }

    #[test]
    fn test_dl_steal_the_stars() {
        let url =
            "https://web.archive.org/web/20180120104957if_/https://rss.art19.com/steal-the-stars";
        // Create and index a source
        let source = Source::from_url(url).unwrap();
        // Copy it's id
        let sid = source.id();
        pipeline::run(vec![source], true).unwrap();

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid).unwrap();
        let title = "Introducing Steal the Stars";
        // Get an episode
        let mut episode = dbqueries::get_episode_from_pk(title, pd.id())
            .unwrap()
            .into();
        let download_fold = get_download_folder(&pd.title()).unwrap();

        get_episode(&mut episode, &download_fold, None).unwrap();

        let final_path = format!("{}/{}.mp3", &download_fold, episode.rowid());
        assert!(Path::new(&final_path).exists());
        fs::remove_file(final_path).unwrap();
    }
}
