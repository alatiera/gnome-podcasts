use failure::Error;
use rayon;

// use podcasts_data::Episode;
use podcasts_data::dbqueries;
use podcasts_downloader::downloader::{get_episode, DownloadProgress};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
// use std::sync::atomic::AtomicUsize;
// use std::path::PathBuf;

// This is messy, undocumented and hacky af.
// I am terrible at writing downloaders and download managers.

#[derive(Debug)]
pub(crate) struct Progress {
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
    pub(crate) fn get_fraction(&self) -> f64 {
        let ratio = self.downloaded_bytes as f64 / self.total_bytes as f64;
        debug!("{:?}", self);
        debug!("Ratio completed: {}", ratio);

        if ratio >= 1.0 {
            return 1.0;
        };
        ratio
    }

    pub(crate) fn get_total_size(&self) -> u64 {
        self.total_bytes
    }

    pub(crate) fn get_downloaded(&self) -> u64 {
        self.downloaded_bytes
    }

    pub(crate) fn cancel(&mut self) {
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
    pub(crate) static ref ACTIVE_DOWNLOADS: Arc<RwLock<HashMap<i32, Arc<Mutex<Progress>>>>> =
        { Arc::new(RwLock::new(HashMap::new())) };
    static ref DLPOOL: rayon::ThreadPool = rayon::ThreadPoolBuilder::new().build().unwrap();
}

pub(crate) fn add(id: i32, directory: String) -> Result<(), Error> {
    // Create a new `Progress` struct to keep track of dl progress.
    let prog = Arc::new(Mutex::new(Progress::default()));

    match ACTIVE_DOWNLOADS.write() {
        Ok(mut guard) => guard.insert(id, prog.clone()),
        Err(err) => return Err(format_err!("ActiveDownloads: {}.", err)),
    };

    DLPOOL.spawn(move || {
        if let Ok(mut episode) = dbqueries::get_episode_widget_from_rowid(id) {
            let id = episode.rowid();

            get_episode(&mut episode, directory.as_str(), Some(prog))
                .map_err(|err| error!("Download Failed: {}", err))
                .ok();

            if let Ok(mut m) = ACTIVE_DOWNLOADS.write() {
                let foo = m.remove(&id);
                debug!("Removed: {:?}", foo);
            }

            // if let Ok(m) = ACTIVE_DOWNLOADS.read() {
            //     debug!("ACTIVE DOWNLOADS: {:#?}", m);
            // }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use podcasts_data::dbqueries;
    use podcasts_data::pipeline;
    use podcasts_data::utils::get_download_folder;
    use podcasts_data::{Episode, Save, Source};

    use podcasts_downloader::downloader::get_episode;

    use std::fs;
    use std::path::Path;
    use std::{thread, time};

    #[test]
    // This test inserts an rss feed to your `XDG_DATA/podcasts/podcasts.db` so we make it explicit
    // to run it.
    #[ignore]
    // THIS IS NOT A RELIABLE TEST
    // Just quick sanity check
    fn test_start_dl() -> Result<(), Error> {
        let url = "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff";
        // Create and index a source
        let mut source = Source::from_url(url)?;
        // Copy its id
        let sid = source.id();
        source.set_http_etag(None);
        source.set_last_modified(None);
        source.save()?;
        pipeline::run(vec![source])?;

        // Get the podcast
        let pd = dbqueries::get_podcast_from_source_id(sid)?;
        let title = "Coming soon... The Tip Off";
        // Get an episode
        let episode: Episode = dbqueries::get_episode_from_pk(title, pd.id())?;

        let download_fold = get_download_folder(&pd.title())?;
        let fold2 = download_fold.clone();
        add(episode.rowid(), download_fold)?;
        assert_eq!(ACTIVE_DOWNLOADS.read().unwrap().len(), 1);

        // Give it some time to download the file
        thread::sleep(time::Duration::from_secs(20));

        let final_path = format!("{}/{}.mp3", &fold2, episode.rowid());
        assert_eq!(ACTIVE_DOWNLOADS.read().unwrap().len(), 0);
        assert!(Path::new(&final_path).exists());
        fs::remove_file(final_path)?;
        Ok(())
    }

    #[test]
    // This test needs access to local system so we ignore it by default.
    #[ignore]
    fn test_dl_steal_the_stars() -> Result<(), Error> {
        let url =
            "https://web.archive.org/web/20180120104957if_/https://rss.art19.com/steal-the-stars";
        // Create and index a source
        let mut source = Source::from_url(url)?;
        // Copy its id
        let sid = source.id();
        source.set_http_etag(None);
        source.set_last_modified(None);
        source.save()?;
        pipeline::run(vec![source])?;

        // Get the podcast
        let pd = dbqueries::get_podcast_from_source_id(sid)?;
        let title = "Introducing Steal the Stars";
        // Get an episode
        let mut episode = dbqueries::get_episode_from_pk(title, pd.id())?.into();
        let download_fold = get_download_folder(&pd.title())?;

        get_episode(&mut episode, &download_fold, None)?;

        let final_path = format!("{}/{}.mp3", &download_fold, episode.rowid());
        assert!(Path::new(&final_path).exists());
        fs::remove_file(final_path)?;
        Ok(())
    }
}
