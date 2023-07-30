// downloader.rs
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

use glob::glob;
use reqwest::header::*;
use reqwest::redirect::Policy;
use tempfile::TempDir;

use std::fs;
use std::fs::{copy, remove_file, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::errors::DownloadError;
use crate::utils;
use crate::xdg_dirs::PODCASTS_CACHE;
use crate::{EpisodeWidgetModel, Save, ShowCoverModel};

// TODO: Replace path that are of type &str with std::path.
// TODO: Have a convention/document absolute/relative paths, if they should end
// with / or not.

pub trait DownloadProgress {
    fn get_downloaded(&self) -> u64;
    fn set_downloaded(&mut self, downloaded: u64);
    fn get_size(&self) -> u64;
    fn set_size(&mut self, bytes: u64);
    fn should_cancel(&self) -> bool;
    fn cancel(&mut self);
}

pub fn client_builder() -> reqwest::ClientBuilder {
    // Haven't included the loop check as
    // Steal the Stars would trigger it as
    // it has a loop back before giving correct url
    let policy = Policy::custom(|attempt| {
        info!("Redirect Attempt URL: {:?}", attempt.url());
        if attempt.previous().len() > 20 {
            attempt.error("too many redirects")
        } else if Some(attempt.url()) == attempt.previous().last() {
            // avoid redirect loops
            attempt.stop()
        } else {
            attempt.follow()
        }
    });

    reqwest::Client::builder()
        .redirect(policy)
        .referer(false)
        .user_agent(crate::USER_AGENT)
}

// Adapted from https://github.com/mattgathu/rget .
// I never wanted to write a custom downloader.
// Sorry to those who will have to work with that code.
// Would much rather use a crate,
// or bindings for a lib like youtube-dl(python),
// But can't seem to find one.
// TODO: Write unit-tests.
async fn download_into(
    dir: &str,
    file_title: &str,
    url: &str,
    progress: Option<Arc<Mutex<dyn DownloadProgress + Send>>>,
) -> Result<String, DownloadError> {
    info!("GET request to: {}", url);

    let client = client_builder().build()?;
    let resp = client.get(url).send().await?;
    info!("Status Resp: {}", resp.status());

    if !resp.status().is_success() {
        if let Some(ref prog) = progress {
            if let Ok(mut m) = prog.lock() {
                m.cancel();
            }
        }

        return Err(DownloadError::UnexpectedResponse(resp.status()));
    }

    let headers = resp.headers().clone();
    let ct_len = headers
        .get(CONTENT_LENGTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|len| len.parse().ok());
    let ct_type = headers
        .get(CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .map(From::from);

    if let Some(ct_len) = ct_len {
        info!("File Length: {}", ct_len);
    }
    if let Some(ct_type) = ct_type {
        info!("Content Type: {}", ct_type);
    }

    let ext = get_ext(ct_type).unwrap_or_else(|| String::from("unknown"));
    info!("Extension: {}", ext);

    // Construct a temp file to save desired content.
    // It has to be a `new_in` instead of new cause rename can't move cross
    // filesystems.
    let tempdir = TempDir::with_prefix_in("temp_download", PODCASTS_CACHE.to_str().unwrap())?;
    let out_file = format!("{}/temp.part", tempdir.path().to_str().unwrap(),);

    if let Some(ct_len) = ct_len {
        if let Some(ref p) = progress {
            if let Ok(mut m) = p.lock() {
                m.set_size(ct_len);
            }
        }
    };

    // Save requested content into the file.
    save_io(&out_file, resp, progress).await?;

    // Construct the desired path.
    let target = format!("{}/{}.{}", dir, file_title, ext);
    // Rename/move the tempfile into a permanent place upon success.
    // Unlike rename(), copy() + remove_file() works even when the
    // temp dir is on a different mount point than the target dir.
    copy(&out_file, &target)?;
    remove_file(out_file)?;
    info!("Downloading of {} completed successfully.", &target);
    Ok(target)
}

/// Determine the file extension from the http content-type header.
fn get_ext(content: Option<&str>) -> Option<String> {
    let mut iter = content?.split('/');
    let type_ = iter.next()?;
    let subtype = iter.next()?;
    mime_guess::get_extensions(type_, subtype).and_then(|c| {
        if c.contains(&subtype) {
            Some(subtype.to_string())
        } else {
            Some(c.first()?.to_string())
        }
    })
}

// TODO: Write unit-tests.
// TODO: Refactor... Somehow.
/// Handles the I/O of fetching a remote file and saving into a Buffer and A
/// File.
async fn save_io(
    file: &str,
    resp: reqwest::Response,
    progress: Option<Arc<Mutex<dyn DownloadProgress + Send>>>,
) -> Result<(), DownloadError> {
    use futures_util::StreamExt;
    use std::ops::Deref;

    info!("Downloading into: {}", file);
    let mut writer = BufWriter::new(File::create(file)?);
    let mut body_stream = resp.bytes_stream();

    while let Some(chunk) = body_stream.next().await {
        if let Ok(chunk) = chunk {
            writer.write_all(chunk.deref())?;
            // This sucks.
            // Actually the whole download module is hack, so w/e.
            if let Some(prog) = progress.clone() {
                let len = writer.get_ref().metadata().map(|x| x.len());
                if let Ok(l) = len {
                    if let Ok(mut m) = prog.lock() {
                        if m.should_cancel() {
                            return Err(DownloadError::DownloadCancelled);
                        }
                        m.set_downloaded(l);
                    }
                }
            }
        } else {
            break;
        }
    }

    Ok(())
}

// TODO: Refactor
pub async fn get_episode(
    ep: &mut EpisodeWidgetModel,
    download_dir: &str,
    progress: Option<Arc<Mutex<dyn DownloadProgress + Send>>>,
) -> Result<(), DownloadError> {
    // Check if its alrdy downloaded
    if ep.local_uri().is_some() {
        if Path::new(ep.local_uri().unwrap()).exists() {
            return Ok(());
        }

        // If the path is not valid, then set it to None.
        ep.set_local_uri(None);
        ep.save()?;
    };

    let path = download_into(
        download_dir,
        &ep.rowid().to_string(),
        ep.uri().unwrap(),
        progress,
    )
    .await?;

    // If download succeeds set episode local_uri to dlpath.
    ep.set_local_uri(Some(&path));

    // Over-write episode length
    let size = fs::metadata(path);
    if let Ok(s) = size {
        ep.set_length(Some(s.len() as i32))
    };

    ep.save()?;
    Ok(())
}

pub fn check_for_cached_cover(pd: &ShowCoverModel) -> Option<PathBuf> {
    let cache_path = utils::get_cover_dir(pd.title()).ok()?;

    // try to match the cover file
    // FIXME: in case the cover changes filetype we will be matching the
    // existing one instead of downloading the new one.
    // Should probably make sure that 'cover.*' is removed when we
    // download new files.
    if let Ok(mut paths) = glob(&format!("{}/cover.*", cache_path)) {
        // Take the first file matching, disregard extension
        let path = paths.next().and_then(|x| x.ok());
        return path;
    }

    None
}

pub fn check_for_cached_image(pd: &ShowCoverModel, uri: &str) -> Option<PathBuf> {
    let cache_path = utils::get_cover_dir(pd.title()).ok()?;
    let hash = utils::calculate_hash(uri);

    if let Ok(mut paths) = glob(&format!("{}/{}.*", hash, cache_path)) {
        // Take the first file matching, disregard extension
        let path = paths.next().and_then(|x| x.ok());
        return path;
    }

    None
}

pub async fn cache_image(pd: &ShowCoverModel) -> Result<String, DownloadError> {
    if let Some(path) = check_for_cached_cover(pd) {
        return Ok(path
            .to_str()
            .ok_or(DownloadError::InvalidCachedImageLocation)?
            .to_owned());
    }

    let url = pd
        .image_uri()
        .ok_or(DownloadError::NoImageLocation)?
        .to_owned();

    if url.is_empty() {
        return Err(DownloadError::NoImageLocation);
    }

    let cache_path = utils::get_cover_dir(pd.title())?;

    let path = download_into(&cache_path, "cover", &url, None).await?;
    info!("Cached img into: {}", &path);
    Ok(path)
}

pub async fn cache_episode_image(
    pd: &ShowCoverModel,
    uri: &str,
    download: bool,
) -> Result<String, DownloadError> {
    if let Some(path) = check_for_cached_image(pd, uri) {
        return Ok(path
            .to_str()
            .ok_or(DownloadError::InvalidCachedImageLocation)?
            .to_owned());
    }

    if uri.is_empty() {
        return Err(DownloadError::NoImageLocation);
    }

    let cache_path = utils::get_cover_dir(pd.title())?;
    let hash = utils::calculate_hash(uri);

    if download {
        let path = download_into(&cache_path, &format!("{}", hash), uri, None).await?;
        info!("Cached img into: {}", &path);
        Ok(path)
    } else {
        Err(DownloadError::DownloadCancelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::pipeline;
    use crate::{dbqueries, Source};
    use anyhow::Result;
    use std::fs;

    #[test]
    // This test inserts an rss feed to your `XDG_DATA/podcasts/podcasts.db` so we make it explicit
    // to run it.
    #[ignore]
    fn test_cache_image() -> Result<()> {
        let url = "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff";
        // Create and index a source
        let source = Source::from_url(url)?;
        // Copy it's id
        let sid = source.id();
        // Convert Source it into a future Feed and index it
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(vec![source]))?;

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid)?.into();

        let img_path = rt.block_on(cache_image(&pd));
        let foo_ = format!(
            "{}{}/cover.jpeg",
            PODCASTS_CACHE.to_str().unwrap(),
            pd.title()
        );
        assert_eq!(img_path?, foo_);
        fs::remove_file(foo_)?;
        Ok(())
    }
}
