//! Helper utilities for accomplishing various tasks.

use chrono::prelude::*;
use rayon::prelude::*;

use url::{Position, Url};

use dbqueries;
use errors::DataError;
use models::{EpisodeCleanerModel, Save, Show};
use xdg_dirs::DL_DIR;

use std::fs;
use std::path::Path;

/// Scan downloaded `episode` entries that might have broken `local_uri`s and
/// set them to `None`.
fn download_checker() -> Result<(), DataError> {
    let mut episodes = dbqueries::get_downloaded_episodes()?;

    episodes
        .par_iter_mut()
        .filter_map(|ep| {
            if !Path::new(ep.local_uri()?).exists() {
                return Some(ep);
            }
            None
        })
        .for_each(|ep| {
            ep.set_local_uri(None);
            ep.save()
                .map_err(|err| error!("{}", err))
                .map_err(|_| error!("Error while trying to update episode: {:#?}", ep))
                .ok();
        });

    Ok(())
}

/// Delete watched `episodes` that have exceded their liftime after played.
fn played_cleaner(cleanup_date: DateTime<Utc>) -> Result<(), DataError> {
    let mut episodes = dbqueries::get_played_cleaner_episodes()?;
    let now_utc = cleanup_date.timestamp() as i32;

    episodes
        .par_iter_mut()
        .filter(|ep| ep.local_uri().is_some() && ep.played().is_some())
        .for_each(|ep| {
            let limit = ep.played().unwrap();
            if now_utc > limit {
                delete_local_content(ep)
                    .map(|_| info!("Episode {:?} was deleted succesfully.", ep.local_uri()))
                    .map_err(|err| error!("Error: {}", err))
                    .map_err(|_| error!("Failed to delete file: {:?}", ep.local_uri()))
                    .ok();
            }
        });
    Ok(())
}

/// Check `ep.local_uri` field and delete the file it points to.
fn delete_local_content(ep: &mut EpisodeCleanerModel) -> Result<(), DataError> {
    if ep.local_uri().is_some() {
        let uri = ep.local_uri().unwrap().to_owned();
        if Path::new(&uri).exists() {
            let res = fs::remove_file(&uri);
            if res.is_ok() {
                ep.set_local_uri(None);
                ep.save()?;
            } else {
                error!("Error while trying to delete file: {}", uri);
                error!("{}", res.unwrap_err());
            };
        }
    } else {
        error!(
            "Something went wrong evaluating the following path: {:?}",
            ep.local_uri(),
        );
    }
    Ok(())
}

/// Database cleaning tasks.
///
/// Runs a download checker which looks for `Episode.local_uri` entries that
/// doesn't exist and sets them to None
///
/// Runs a cleaner for played Episode's that are pass the lifetime limit and
/// scheduled for removal.
pub fn checkup(cleanup_date: DateTime<Utc>) -> Result<(), DataError> {
    info!("Running database checks.");
    download_checker()?;
    played_cleaner(cleanup_date)?;
    info!("Checks completed.");
    Ok(())
}

/// Remove fragment identifiers and query pairs from a URL
/// If url parsing fails, return's a trimmed version of the original input.
pub fn url_cleaner(s: &str) -> String {
    // Copied from the cookbook.
    // https://rust-lang-nursery.github.io/rust-cookbook/net.html
    // #remove-fragment-identifiers-and-query-pairs-from-a-url
    match Url::parse(s) {
        Ok(parsed) => parsed[..Position::AfterPath].to_owned(),
        _ => s.trim().to_owned(),
    }
}

/// Returns the URI of a Show Downloads given it's title.
pub fn get_download_folder(pd_title: &str) -> Result<String, DataError> {
    // It might be better to make it a hash of the title or the Show rowid
    let download_fold = format!("{}/{}", DL_DIR.to_str().unwrap(), pd_title);

    // Create the folder
    fs::DirBuilder::new()
        .recursive(true)
        .create(&download_fold)?;
    Ok(download_fold)
}

/// Removes all the entries associated with the given show from the database,
/// and deletes all of the downloaded content.
// TODO: Write Tests
pub fn delete_show(pd: &Show) -> Result<(), DataError> {
    dbqueries::remove_feed(pd)?;
    info!("{} was removed succesfully.", pd.title());

    let fold = get_download_folder(pd.title())?;
    fs::remove_dir_all(&fold)?;
    info!("All the content at, {} was removed succesfully", &fold);
    Ok(())
}

#[cfg(test)]
use Feed;

#[cfg(test)]
/// Helper function that open a local file, parse the rss::Channel and gives back a Feed object.
/// Alternative Feed constructor to be used for tests.
pub fn get_feed(file_path: &str, id: i32) -> Feed {
    use feed::FeedBuilder;
    use rss::Channel;
    use std::fs;
    use std::io::BufReader;

    // open the xml file
    let feed = fs::File::open(file_path).unwrap();
    // parse it into a channel
    let chan = Channel::read_from(BufReader::new(feed)).unwrap();
    FeedBuilder::default()
        .channel(chan)
        .source_id(id)
        .build()
        .unwrap()
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use self::tempdir::TempDir;
    use super::*;
    use chrono::Duration;

    use database::truncate_db;
    use models::NewEpisodeBuilder;

    use std::fs::File;
    use std::io::Write;

    fn helper_db() -> TempDir {
        // Clean the db
        truncate_db().unwrap();
        // Setup tmp file stuff
        let tmp_dir = TempDir::new("podcasts_test").unwrap();
        let valid_path = tmp_dir.path().join("virtual_dl.mp3");
        let bad_path = tmp_dir.path().join("invalid_thing.mp3");
        let mut tmp_file = File::create(&valid_path).unwrap();
        writeln!(tmp_file, "Foooo").unwrap();

        // Setup episodes
        let n1 = NewEpisodeBuilder::default()
            .title("foo_bar".to_string())
            .show_id(0)
            .build()
            .unwrap()
            .to_episode()
            .unwrap();

        let n2 = NewEpisodeBuilder::default()
            .title("bar_baz".to_string())
            .show_id(1)
            .build()
            .unwrap()
            .to_episode()
            .unwrap();

        let mut ep1 = dbqueries::get_episode_from_pk(n1.title(), n1.show_id()).unwrap();
        let mut ep2 = dbqueries::get_episode_from_pk(n2.title(), n2.show_id()).unwrap();
        ep1.set_local_uri(Some(valid_path.to_str().unwrap()));
        ep2.set_local_uri(Some(bad_path.to_str().unwrap()));

        ep1.save().unwrap();
        ep2.save().unwrap();

        tmp_dir
    }

    #[test]
    fn test_download_checker() {
        let tmp_dir = helper_db();
        download_checker().unwrap();
        let episodes = dbqueries::get_downloaded_episodes().unwrap();
        let valid_path = tmp_dir.path().join("virtual_dl.mp3");

        assert_eq!(episodes.len(), 1);
        assert_eq!(
            Some(valid_path.to_str().unwrap()),
            episodes.first().unwrap().local_uri()
        );

        let _tmp_dir = helper_db();
        download_checker().unwrap();
        let episode = dbqueries::get_episode_from_pk("bar_baz", 1).unwrap();
        assert!(episode.local_uri().is_none());
    }

    #[test]
    fn test_download_cleaner() {
        let _tmp_dir = helper_db();
        let mut episode: EpisodeCleanerModel =
            dbqueries::get_episode_from_pk("foo_bar", 0).unwrap().into();

        let valid_path = episode.local_uri().unwrap().to_owned();
        delete_local_content(&mut episode).unwrap();
        assert_eq!(Path::new(&valid_path).exists(), false);
    }

    #[test]
    fn test_played_cleaner_expired() {
        let _tmp_dir = helper_db();
        let mut episode = dbqueries::get_episode_from_pk("foo_bar", 0).unwrap();
        let cleanup_date = Utc::now() - Duration::seconds(1000);
        let epoch = cleanup_date.timestamp() as i32 - 1;
        episode.set_played(Some(epoch));
        episode.save().unwrap();
        let valid_path = episode.local_uri().unwrap().to_owned();

        // This should delete the file
        played_cleaner(cleanup_date).unwrap();
        assert_eq!(Path::new(&valid_path).exists(), false);
    }

    #[test]
    fn test_played_cleaner_none() {
        let _tmp_dir = helper_db();
        let mut episode = dbqueries::get_episode_from_pk("foo_bar", 0).unwrap();
        let cleanup_date = Utc::now() - Duration::seconds(1000);
        let epoch = cleanup_date.timestamp() as i32 + 1;
        episode.set_played(Some(epoch));
        episode.save().unwrap();
        let valid_path = episode.local_uri().unwrap().to_owned();

        // This should not delete the file
        played_cleaner(cleanup_date).unwrap();
        assert_eq!(Path::new(&valid_path).exists(), true);
    }

    #[test]
    fn test_url_cleaner() {
        let good_url = "http://traffic.megaphone.fm/FL8608731318.mp3";
        let bad_url = "http://traffic.megaphone.fm/FL8608731318.mp3?updated=1484685184";

        assert_eq!(url_cleaner(bad_url), good_url);
        assert_eq!(url_cleaner(good_url), good_url);
        assert_eq!(url_cleaner(&format!("   {}\t\n", bad_url)), good_url);
    }

    #[test]
    // This test needs access to local system so we ignore it by default.
    #[ignore]
    fn test_get_dl_folder() {
        let foo_ = format!("{}/{}", DL_DIR.to_str().unwrap(), "foo");
        assert_eq!(get_download_folder("foo").unwrap(), foo_);
        let _ = fs::remove_dir_all(foo_);
    }
}
