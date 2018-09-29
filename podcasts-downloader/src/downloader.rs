use glob::glob;
use mime_guess;
use reqwest;
use reqwest::RedirectPolicy;
use reqwest::header::*;
use tempdir::TempDir;

use std::fs;
use std::fs::{rename, DirBuilder, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use podcasts_data::xdg_dirs::PODCASTS_CACHE;
use podcasts_data::{EpisodeWidgetModel, Save, ShowCoverModel};

// use failure::Error;
use errors::DownloadError;

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
// But cant seem to find one.
// TODO: Write unit-tests.
fn download_into(
    dir: &str,
    file_title: &str,
    url: &str,
    progress: Option<Arc<Mutex<dyn DownloadProgress>>>,
) -> Result<String, DownloadError> {
    info!("GET request to: {}", url);
    // Haven't included the loop check as
    // Steal the Stars would tigger it as
    // it has a loop back before giving correct url
    let policy = RedirectPolicy::custom(|attempt| {
        info!("Redirect Attempt URL: {:?}", attempt.url());
        if attempt.previous().len() > 5 {
            attempt.too_many_redirects()
        } else if Some(attempt.url()) == attempt.previous().last() {
            attempt.loop_detected()
        } else {
            attempt.follow()
        }
    });

    let client = reqwest::Client::builder()
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
    let ct_len = headers.get::<ContentLength>().map(|ct_len| **ct_len);
    let ct_type = headers.get::<ContentType>();
    ct_len.map(|x| info!("File Lenght: {}", x));
    ct_type.map(|x| info!("Content Type: {}", x));

    let ext = get_ext(ct_type.cloned()).unwrap_or_else(|| String::from("unknown"));
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
    info!("Downloading of {} completed succesfully.", &target);
    Ok(target)
}

/// Determine the file extension from the http content-type header.
fn get_ext(content: Option<ContentType>) -> Option<String> {
    let cont = content.clone()?;
    content
        .and_then(|c| mime_guess::get_extensions(c.type_().as_ref(), c.subtype().as_ref()))
        .and_then(|c| {
            if c.contains(&cont.subtype().as_ref()) {
                Some(cont.subtype().as_ref().to_string())
            } else {
                Some(c.first().unwrap().to_string())
            }
        })
}

// TODO: Write unit-tests.
// TODO: Refactor... Somehow.
/// Handles the I/O of fetching a remote file and saving into a Buffer and A
/// File.
#[allow(needless_pass_by_value)]
fn save_io(
    file: &str,
    resp: &mut reqwest::Response,
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

    // If download succedes set episode local_uri to dlpath.
    ep.set_local_uri(Some(&path));

    // Over-write episode lenght
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
    use failure::Error;
    use podcasts_data::dbqueries;
    use podcasts_data::pipeline;
    use podcasts_data::Source;

    use std::fs;

    #[test]
    // This test inserts an rss feed to your `XDG_DATA/podcasts/podcasts.db` so we make it explicit
    // to run it.
    #[ignore]
    fn test_cache_image() -> Result<(), Error> {
        let url = "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff";
        // Create and index a source
        let source = Source::from_url(url)?;
        // Copy it's id
        let sid = source.id();
        // Convert Source it into a future Feed and index it
        pipeline::run(vec![source])?;

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
