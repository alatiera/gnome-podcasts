// utils.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Helper utilities for accomplishing various tasks.

use chrono::prelude::*;

use url::{Position, Url};

use crate::dbqueries;
use crate::errors::{DataError, DownloadError};
use crate::models::{EpisodeCleanerModel, Save, Show};
use crate::xdg_dirs::{DL_DIR, PODCASTS_CACHE};

use std::fs;
use std::path::Path;

/// Convert a `u64` to a `Vec<u8>`.
///
/// This function is used to convert hash values into a format suitable for the database, i.e. `Vec<u8>`.
/// The resulting vector will always have exactly 8 values.
/// The individual bytes are extracted from the given `u64`, which is parsed as little-endian.
pub fn u64_to_vec_u8(u: u64) -> Vec<u8> {
    let bytes: Vec<u8> = u.to_le_bytes().to_vec();
    debug_assert_eq!(bytes.len(), 8);
    bytes
}

/// Convert a `Vec<u8>` of bytes to a `u64`.
///
/// These values together should represent a `u64` value in little-endian byte order.
///
/// # Panics
///
/// The given vector must have exactly 8 elements otherwise it will panic.
///
pub fn vec_u8_to_u64(v: Vec<u8>) -> u64 {
    assert_eq!(v.len(), 8);
    u64::from_le_bytes(v[..].try_into().unwrap())
}

/// Hash a given value.
pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

/// Scan downloaded `episode` entries that might have broken `local_uri`s and
/// set them to `None`.
async fn download_checker(runtime: &tokio::runtime::Runtime) -> Result<(), DataError> {
    let episodes = dbqueries::get_downloaded_episodes()?;

    let handles = episodes
        .into_iter()
        .filter_map(|ep| {
            if !Path::new(ep.local_uri()?).exists() {
                return Some(ep);
            }
            None
        })
        .map(|ep| runtime.spawn(update_download_status(ep)));

    futures::future::join_all(handles).await;
    Ok(())
}

async fn update_download_status(mut ep: EpisodeCleanerModel) {
    ep.set_local_uri(None);
    ep.save()
        .map_err(|err| error!("{}", err))
        .map_err(|_| error!("Error while trying to update episode: {:#?}", ep))
        .ok();
}

/// Delete watched `episodes` that have exceeded their lifetime after played.
async fn played_cleaner(
    runtime: &tokio::runtime::Runtime,
    cleanup_date: DateTime<Utc>,
) -> Result<(), DataError> {
    let episodes = dbqueries::get_played_cleaner_episodes()?;
    let now_utc = cleanup_date.timestamp() as i32;

    let handles = episodes
        .into_iter()
        .filter(|ep| ep.local_uri().is_some() && ep.played().is_some())
        .map(|ep| runtime.spawn(clean_played(now_utc, ep)));
    futures::future::join_all(handles).await;
    Ok(())
}

async fn clean_played(now_utc: i32, mut ep: EpisodeCleanerModel) {
    let limit = ep.played().unwrap();
    if now_utc > limit {
        delete_local_content(&mut ep)
            .map(|_| info!("Episode {:?} was deleted successfully.", ep.local_uri()))
            .map_err(|err| error!("Error: {}", err))
            .map_err(|_| error!("Failed to delete file: {:?}", ep.local_uri()))
            .ok();
    }
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
pub async fn checkup(
    runtime: &tokio::runtime::Runtime,
    cleanup_date: DateTime<Utc>,
) -> Result<(), DataError> {
    info!("Running database checks.");
    download_checker(runtime).await?;
    played_cleaner(runtime, cleanup_date).await?;
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
        Ok(parsed) => parsed[..Position::AfterQuery].to_owned(),
        _ => s.trim().to_owned(),
    }
}

/// Returns the URI of a Show' Download directory given it's title.
pub fn get_download_dir(pd_title: &str) -> Result<String, DownloadError> {
    // It might be better to make it a hash of the title or the Show rowid
    let mut dir = DL_DIR.clone();
    dir.push(pd_title);

    // Create the dir
    fs::DirBuilder::new().recursive(true).create(&dir)?;
    let dir_str = dir.to_str().ok_or(DownloadError::InvalidCacheLocation)?;
    Ok(dir_str.to_owned())
}

/// Returns the URI of a Show's cover directory given it's title.
pub fn get_cover_dir(pd_title: &str) -> Result<String, DownloadError> {
    // It might be better to make it a hash of the title or the Show rowid
    let mut dir = PODCASTS_CACHE.clone();
    dir.push(pd_title);

    // Create the dir
    fs::DirBuilder::new().recursive(true).create(&dir)?;
    let dir_str = dir
        .to_str()
        .ok_or(DownloadError::InvalidCachedImageLocation)?;
    Ok(dir_str.to_owned())
}

/// Removes all the entries associated with the given show from the database,
/// and deletes all of the downloaded content.
// TODO: Write Tests
pub fn delete_show(pd: &Show) -> Result<(), DownloadError> {
    dbqueries::remove_feed(pd)?;
    info!("{} was removed successfully.", pd.title());

    let download_dir = get_download_dir(pd.title())?;
    fs::remove_dir_all(&download_dir)?;
    info!(
        "All the episodes at, {} was removed successfully",
        &download_dir
    );

    let cover_dir = get_cover_dir(pd.title())?;
    fs::remove_dir_all(&cover_dir)?;
    info!("All the Covers at, {} was removed successfully", &cover_dir);
    Ok(())
}

#[cfg(test)]
use crate::Feed;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryInto;
use std::hash::{Hash, Hasher};

#[cfg(test)]
/// Helper function that open a local file, parse the rss::Channel and gives back a Feed object.
/// Alternative Feed constructor to be used for tests.
pub fn get_feed(file_path: &str, id: i32) -> Feed {
    use crate::feed::FeedBuilder;
    use rss::Channel;
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
    use super::*;
    use anyhow::Result;
    use chrono::Duration;
    use tempdir::TempDir;

    use crate::database::truncate_db;
    use crate::models::NewEpisodeBuilder;

    use std::fs::File;
    use std::io::Write;

    fn helper_db() -> Result<TempDir> {
        // Clean the db
        truncate_db()?;
        // Setup tmp file stuff
        let tmp_dir = TempDir::new("podcasts_test")?;
        let valid_path = tmp_dir.path().join("virtual_dl.mp3");
        let bad_path = tmp_dir.path().join("invalid_thing.mp3");
        let mut tmp_file = File::create(&valid_path)?;
        writeln!(tmp_file, "Foooo")?;

        // Setup episodes
        let n1 = NewEpisodeBuilder::default()
            .title("foo_bar".to_string())
            .show_id(0)
            .build()
            .unwrap()
            .to_episode()?;

        let n2 = NewEpisodeBuilder::default()
            .title("bar_baz".to_string())
            .show_id(1)
            .build()
            .unwrap()
            .to_episode()?;

        let mut ep1 = dbqueries::get_episode_cleaner_from_pk(n1.title(), n1.show_id())?;
        let mut ep2 = dbqueries::get_episode_cleaner_from_pk(n2.title(), n2.show_id())?;
        ep1.set_local_uri(Some(valid_path.to_str().unwrap()));
        ep2.set_local_uri(Some(bad_path.to_str().unwrap()));

        ep1.save()?;
        ep2.save()?;

        Ok(tmp_dir)
    }

    #[test]
    fn test_download_checker() -> Result<()> {
        let tmp_dir = helper_db()?;
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(download_checker(&rt))?;
        let episodes = dbqueries::get_downloaded_episodes()?;
        let valid_path = tmp_dir.path().join("virtual_dl.mp3");

        assert_eq!(episodes.len(), 1);
        assert_eq!(
            Some(valid_path.to_str().unwrap()),
            episodes.first().unwrap().local_uri()
        );

        let _tmp_dir = helper_db()?;
        rt.block_on(download_checker(&rt))?;
        let episode = dbqueries::get_episode_cleaner_from_pk("bar_baz", 1)?;
        assert!(episode.local_uri().is_none());
        Ok(())
    }

    #[test]
    fn test_download_cleaner() -> Result<()> {
        let _tmp_dir = helper_db()?;
        let mut episode: EpisodeCleanerModel =
            dbqueries::get_episode_cleaner_from_pk("foo_bar", 0)?;

        let valid_path = episode.local_uri().unwrap().to_owned();
        delete_local_content(&mut episode)?;
        assert!(!Path::new(&valid_path).exists());
        Ok(())
    }

    #[test]
    fn test_played_cleaner_expired() -> Result<()> {
        let _tmp_dir = helper_db()?;
        let mut episode = dbqueries::get_episode_cleaner_from_pk("foo_bar", 0)?;
        let cleanup_date = Utc::now() - Duration::seconds(1000);
        let epoch = cleanup_date.timestamp() as i32 - 1;
        episode.set_played(Some(epoch));
        episode.save()?;
        let valid_path = episode.local_uri().unwrap().to_owned();
        let rt = tokio::runtime::Runtime::new()?;

        // This should delete the file
        rt.block_on(played_cleaner(&rt, cleanup_date))?;
        assert!(!Path::new(&valid_path).exists());
        Ok(())
    }

    #[test]
    fn test_played_cleaner_none() -> Result<()> {
        let _tmp_dir = helper_db()?;
        let mut episode = dbqueries::get_episode_cleaner_from_pk("foo_bar", 0)?;
        let cleanup_date = Utc::now() - Duration::seconds(1000);
        let epoch = cleanup_date.timestamp() as i32 + 1;
        episode.set_played(Some(epoch));
        episode.save()?;
        let valid_path = episode.local_uri().unwrap().to_owned();
        let rt = tokio::runtime::Runtime::new()?;

        // This should not delete the file
        rt.block_on(played_cleaner(&rt, cleanup_date))?;
        assert!(Path::new(&valid_path).exists());
        Ok(())
    }

    #[test]
    fn test_url_cleaner() -> Result<()> {
        let good_url = "http://traffic.megaphone.fm/FL8608731318.mp3?updated=1484685184";
        let bad_url = "http://traffic.megaphone.fm/FL8608731318.mp3?updated=1484685184#foobar";

        assert_eq!(url_cleaner(bad_url), good_url);
        assert_eq!(url_cleaner(good_url), good_url);
        assert_eq!(url_cleaner(&format!("   {}\t\n", bad_url)), good_url);
        Ok(())
    }

    #[test]
    // This test needs access to local system so we ignore it by default.
    #[ignore]
    fn test_get_dl_dir() -> Result<()> {
        let foo_ = format!("{}/{}", DL_DIR.to_str().unwrap(), "foo");
        assert_eq!(get_download_dir("foo")?, foo_);
        let _ = fs::remove_dir_all(foo_);
        Ok(())
    }

    #[test]
    fn hash_should_be_the_same_given_the_same_input() -> Result<()> {
        let image_uri =
            "http://www.jupiterbroadcasting.com/wp-content/uploads/2018/01/lup-0232-v.jpg";
        let first_hash = calculate_hash(&image_uri);
        let second_hash = calculate_hash(&image_uri);
        assert_eq!(first_hash, second_hash);
        Ok(())
    }

    #[test]
    fn hash_should_be_different_for_different_inputs() -> Result<()> {
        let old_image_uri =
            "http://www.jupiterbroadcasting.com/wp-content/uploads/2018/01/lup-0232-v.jpg";
        let new_image_uri = "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3";
        let old_hash = calculate_hash(&old_image_uri);
        let new_hash = calculate_hash(&new_image_uri);
        assert_ne!(old_hash, new_hash);
        Ok(())
    }

    #[test]
    fn hash_should_be_different_for_similar_inputs() -> Result<()> {
        let image_uri_v2 =
            "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=2";
        let image_uri_v3 = "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3";
        let v2_hash = calculate_hash(&image_uri_v2);
        let v3_hash = calculate_hash(&image_uri_v3);
        assert_ne!(v2_hash, v3_hash);
        Ok(())
    }

    #[test]
    fn u64_to_vec_u8_should_convert() -> Result<()> {
        assert_eq!(
            u64_to_vec_u8(16358564451669550783),
            vec![191, 166, 24, 137, 178, 75, 5, 227]
        );
        Ok(())
    }

    #[test]
    fn u64_to_vec_u8_should_produce_a_vector_of_exactly_8_elements() -> Result<()> {
        assert_eq!(u64_to_vec_u8(u64::MAX).len(), 8);
        Ok(())
    }

    #[test]
    fn vec_u8_to_u64_should_convert() -> Result<()> {
        assert_eq!(
            vec_u8_to_u64(vec![0, 1, 2, 3, 4, 5, 6, 7]),
            506097522914230528
        );
        Ok(())
    }

    #[test]
    #[should_panic(expected = "8")]
    fn vec_u8_to_u64_should_panic_given_an_empty_vector() {
        vec_u8_to_u64(vec![]);
    }

    #[test]
    #[should_panic(expected = "8")]
    fn vec_u8_to_u64_should_panic_given_a_vector_with_1_element() {
        vec_u8_to_u64(vec![1]);
    }

    #[test]
    #[should_panic(expected = "8")]
    fn vec_u8_to_u64_should_panic_given_a_vector_with_7_elements() {
        vec_u8_to_u64(vec![10; 7]);
    }

    #[test]
    #[should_panic(expected = "8")]
    fn vec_u8_to_u64_should_panic_given_a_vector_with_9_elements() {
        vec_u8_to_u64(vec![12; 9]);
    }

    #[test]
    fn vec_u8_to_u64_should_be_the_inverse_of_u64_to_vec_u8() -> Result<()> {
        assert_eq!(10, vec_u8_to_u64(u64_to_vec_u8(10)));
        Ok(())
    }
}
