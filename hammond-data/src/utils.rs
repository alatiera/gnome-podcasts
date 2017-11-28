//! Helper utilities for accomplishing various tasks.

use rayon::prelude::*;
use chrono::prelude::*;

use url::{Position, Url};

use errors::*;
use dbqueries;
use models::queryables::Episode;

use std::path::Path;
use std::fs;

fn download_checker() -> Result<()> {
    let episodes = dbqueries::get_downloaded_episodes()?;

    episodes
        .into_par_iter()
        .for_each(|mut ep| checker_helper(&mut ep));

    Ok(())
}

fn checker_helper(ep: &mut Episode) {
    if !Path::new(ep.local_uri().unwrap()).exists() {
        ep.set_local_uri(None);
        let res = ep.save();
        if let Err(err) = res {
            error!("Error while trying to update episode: {:#?}", ep);
            error!("Error: {}", err);
        };
    }
}

fn played_cleaner() -> Result<()> {
    let episodes = dbqueries::get_played_episodes()?;

    let now_utc = Utc::now().timestamp() as i32;
    episodes.into_par_iter().for_each(|mut ep| {
        if ep.local_uri().is_some() && ep.played().is_some() {
            let played = ep.played().unwrap();
            // TODO: expose a config and a user set option.
            // Chnage the test too when exposed
            let limit = played + 172_800; // add 2days in seconds
            if now_utc > limit {
                let e = delete_local_content(&mut ep);
                if let Err(err) = e {
                    error!("Error while trying to delete file: {:?}", ep.local_uri());
                    error!("Error: {}", err);
                } else {
                    info!("Episode {:?} was deleted succesfully.", ep.title());
                };
            }
        }
    });
    Ok(())
}

/// Check `ep.local_uri` field and delete the file it points to.
pub fn delete_local_content(ep: &mut Episode) -> Result<()> {
    if ep.local_uri().is_some() {
        let uri = ep.local_uri().unwrap().to_owned();
        if Path::new(&uri).exists() {
            let res = fs::remove_file(&uri);
            if res.is_ok() {
                ep.set_local_uri(None);
                ep.save()?;
            } else {
                error!("Error while trying to delete file: {}", uri);
                error!("Error: {}", res.unwrap_err());
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
pub fn checkup() -> Result<()> {
    download_checker()?;
    played_cleaner()?;
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

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use super::*;
    use database::{connection, truncate_db};
    use models::insertables::NewEpisodeBuilder;
    use self::tempdir::TempDir;
    use std::fs::File;
    use std::io::Write;

    fn helper_db() -> TempDir {
        // Clean the db
        truncate_db().unwrap();
        // Setup tmp file stuff
        let tmp_dir = TempDir::new("hammond_test").unwrap();
        let valid_path = tmp_dir.path().join("virtual_dl.mp3");
        let bad_path = tmp_dir.path().join("invalid_thing.mp3");
        let mut tmp_file = File::create(&valid_path).unwrap();
        writeln!(tmp_file, "Foooo").unwrap();

        // Setup episodes
        let db = connection();
        let con = db.get().unwrap();
        NewEpisodeBuilder::new()
            .uri("foo_bar".to_string())
            .local_uri(Some(valid_path.to_str().unwrap().to_owned()))
            .build()
            .into_episode(&con)
            .unwrap();

        NewEpisodeBuilder::new()
            .uri("bar_baz".to_string())
            .local_uri(Some(bad_path.to_str().unwrap().to_owned()))
            .build()
            .into_episode(&con)
            .unwrap();

        tmp_dir
    }

    #[test]
    fn test_download_checker() {
        let _tmp_dir = helper_db();
        download_checker().unwrap();
        let episodes = dbqueries::get_downloaded_episodes().unwrap();

        assert_eq!(episodes.len(), 1);
        assert_eq!("foo_bar", episodes.first().unwrap().uri());
    }

    #[test]
    fn test_checker_helper() {
        let _tmp_dir = helper_db();
        let mut episode = {
            let db = connection();
            let con = db.get().unwrap();
            dbqueries::get_episode_from_uri(&con, "bar_baz").unwrap()
        };

        checker_helper(&mut episode);
        assert!(episode.local_uri().is_none());
    }

    #[test]
    fn test_download_cleaner() {
        let _tmp_dir = helper_db();
        let mut episode = {
            let db = connection();
            let con = db.get().unwrap();
            dbqueries::get_episode_from_uri(&con, "foo_bar").unwrap()
        };

        let valid_path = episode.local_uri().unwrap().to_owned();
        delete_local_content(&mut episode).unwrap();
        assert_eq!(Path::new(&valid_path).exists(), false);
    }

    #[test]
    fn test_played_cleaner_expired() {
        let _tmp_dir = helper_db();
        let mut episode = {
            let db = connection();
            let con = db.get().unwrap();
            dbqueries::get_episode_from_uri(&con, "foo_bar").unwrap()
        };
        let now_utc = Utc::now().timestamp() as i32;
        // let limit = now_utc - 172_800;
        let epoch = now_utc - 200_000;
        episode.set_played(Some(epoch));
        episode.save().unwrap();
        let valid_path = episode.local_uri().unwrap().to_owned();

        // This should delete the file
        played_cleaner().unwrap();
        assert_eq!(Path::new(&valid_path).exists(), false);
    }

    #[test]
    fn test_played_cleaner_none() {
        let _tmp_dir = helper_db();
        let mut episode = {
            let db = connection();
            let con = db.get().unwrap();
            dbqueries::get_episode_from_uri(&con, "foo_bar").unwrap()
        };
        let now_utc = Utc::now().timestamp() as i32;
        // limit = 172_800;
        let epoch = now_utc - 20_000;
        episode.set_played(Some(epoch));
        episode.save().unwrap();
        let valid_path = episode.local_uri().unwrap().to_owned();

        // This should not delete the file
        played_cleaner().unwrap();
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
}
