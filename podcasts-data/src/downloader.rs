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
use mime_guess;
use reqwest;
use reqwest::header::*;
use reqwest::redirect::Policy;
use tempdir::TempDir;

use std::fs;
use std::fs::{rename, DirBuilder, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::errors::DownloadError;
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

// Adapted from https://github.com/mattgathu/rget .
// I never wanted to write a custom downloader.
// Sorry to those who will have to work with that code.
// Would much rather use a crate,
// or bindings for a lib like youtube-dl(python),
// But can't seem to find one.
// TODO: Write unit-tests.
fn download_into(
    dir: &str,
    file_title: &str,
    url: &str,
    progress: Option<Arc<Mutex<dyn DownloadProgress>>>,
) -> Result<String, DownloadError> {
    info!("GET request to: {}", url);
    // Haven't included the loop check as
    // Steal the Stars would trigger it as
    // it has a loop back before giving correct url
    let policy = Policy::custom(|attempt| {
        info!("Redirect Attempt URL: {:?}", attempt.url());
        if attempt.previous().len() > 5 {
            attempt.error("too many redirects")
        } else if Some(attempt.url()) == attempt.previous().last() {
            // avoid redirect loops
            attempt.stop()
        } else {
            attempt.follow()
        }
    });

    let client = reqwest::blocking::Client::builder()
        .redirect(policy)
        .referer(false)
        .build()?;
    let mut resp = client.get(url).send()?;
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

    ct_len.map(|x| info!("File Length: {}", x));
    ct_type.map(|x| info!("Content Type: {}", x));

    let ext = get_ext(ct_type).unwrap_or_else(|| String::from("unknown"));
    info!("Extension: {}", ext);

    // Construct a temp file to save desired content.
    // It has to be a `new_in` instead of new cause rename can't move cross
    // filesystems.
    let tempdir = TempDir::new_in(PODCASTS_CACHE.to_str().unwrap(), "temp_download")?;
    let out_file = format!("{}/temp.part", tempdir.path().to_str().unwrap(),);

    ct_len.map(|x| {
        if let Some(ref p) = progress {
            if let Ok(mut m) = p.lock() {
                m.set_size(x);
            }
        }
    });

    // Save requested content into the file.
    save_io(&out_file, &mut resp, ct_len, progress)?;

    // Construct the desired path.
    let target = format!("{}/{}.{}", dir, file_title, ext);
    // Rename/move the tempfile into a permanent place upon success.
    rename(out_file, &target)?;
    info!("Downloading of {} completed successfully.", &target);
    Ok(target)
}

/// Determine the file extension from the http content-type header.
fn get_ext(content: Option<&str>) -> Option<String> {
    let mut iter = content?.split("/");
    let type_ = iter.next()?;
    let subtype = iter.next()?;
    mime_guess::get_extensions(type_, subtype).and_then(|c| {
        if c.contains(&&subtype) {
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
#[allow(clippy::needless_pass_by_value)]
fn save_io(
    file: &str,
    resp: &mut reqwest::blocking::Response,
    content_lenght: Option<u64>,
    progress: Option<Arc<Mutex<dyn DownloadProgress>>>,
) -> Result<(), DownloadError> {
    info!("Downloading into: {}", file);
    let chunk_size = match content_lenght {
        Some(x) => x as usize / 99,
        None => 1024, // default chunk size
    };

    let mut writer = BufWriter::new(File::create(&file)?);

    loop {
        let mut buffer = vec![0; chunk_size];
        let bcount = resp.read(&mut buffer[..])?;
        buffer.truncate(bcount);
        if !buffer.is_empty() {
            writer.write_all(buffer.as_slice())?;
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
pub fn get_episode(
    ep: &mut EpisodeWidgetModel,
    download_folder: &str,
    progress: Option<Arc<Mutex<dyn DownloadProgress>>>,
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
        download_folder,
        &ep.rowid().to_string(),
        ep.uri().unwrap(),
        progress,
    )?;

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

pub fn cache_image(pd: &ShowCoverModel) -> Result<String, DownloadError> {
    let url = pd
        .image_uri()
        .ok_or_else(|| DownloadError::NoImageLocation)?
        .to_owned();

    if url == "" {
        return Err(DownloadError::NoImageLocation);
    }

    let cache_path = PODCASTS_CACHE
        .to_str()
        .ok_or_else(|| DownloadError::InvalidCacheLocation)?;
    let cache_download_fold = format!("{}{}", cache_path, pd.title().to_owned());

    // Weird glob magic.
    if let Ok(mut foo) = glob(&format!("{}/cover.*", cache_download_fold)) {
        // For some reason there is no .first() method so nth(0) is used
        let path = foo.nth(0).and_then(|x| x.ok());
        if let Some(p) = path {
            return Ok(p
                .to_str()
                .ok_or_else(|| DownloadError::InvalidCachedImageLocation)?
                .into());
        }
    };

    // Create the folders if they don't exist.
    DirBuilder::new()
        .recursive(true)
        .create(&cache_download_fold)?;

    let path = download_into(&cache_download_fold, "cover", &url, None)?;
    info!("Cached img into: {}", &path);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbqueries;
    use crate::pipeline::pipeline;
    use crate::Source;
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
        let mut rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(vec![source], None));

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid)?.into();

        let img_path = cache_image(&pd);
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
