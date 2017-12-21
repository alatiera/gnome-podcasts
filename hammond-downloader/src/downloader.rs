use reqwest;
use hyper::header::*;
use tempdir::TempDir;
use mime_guess;

use std::fs::{rename, DirBuilder, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::fs;

use errors::*;
use hammond_data::{EpisodeWidgetQuery, PodcastCoverQuery};
use hammond_data::xdg_dirs::{DL_DIR, HAMMOND_CACHE};

// TODO: Replace path that are of type &str with std::path.
// TODO: Have a convention/document absolute/relative paths, if they should end with / or not.

// Adapted from https://github.com/mattgathu/rget .
// I never wanted to write a custom downloader.
// Sorry to those who will have to work with that code.
// Would much rather use a crate,
// or bindings for a lib like youtube-dl(python),
// But cant seem to find one.
// TODO: Write unit-tests.
fn download_into(dir: &str, file_title: &str, url: &str) -> Result<String> {
    info!("GET request to: {}", url);
    let client = reqwest::Client::builder().referer(false).build()?;
    let mut resp = client.get(url).send()?;
    info!("Status Resp: {}", resp.status());

    if !resp.status().is_success() {
        bail!("Unexpected server response: {}", resp.status())
    }

    let headers = resp.headers().clone();

    let ct_len = headers.get::<ContentLength>().map(|ct_len| **ct_len);
    let ct_type = headers.get::<ContentType>();
    ct_len.map(|x| info!("File Lenght: {}", x));
    ct_type.map(|x| info!("Content Type: {}", x));

    let ext = get_ext(ct_type.cloned()).unwrap_or(String::from("unkown"));
    info!("Extension: {}", ext);

    // Construct a temp file to save desired content.
    let tempdir = TempDir::new_in(dir, "")?;

    let out_file = format!("{}/temp.part", tempdir.path().to_str().unwrap(),);

    // Save requested content into the file.
    save_io(&out_file, &mut resp, ct_len)?;

    // Construct the desired path.
    let target = format!("{}/{}.{}", dir, file_title, ext);
    // Rename/move the tempfile into a permanent place upon success.
    rename(out_file, &target)?;
    info!("Downloading of {} completed succesfully.", &target);
    Ok(target)
}

// Determine the file extension from the http content-type header.
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
/// Handles the I/O of fetching a remote file and saving into a Buffer and A File.
fn save_io(file: &str, resp: &mut reqwest::Response, content_lenght: Option<u64>) -> Result<()> {
    info!("Downloading into: {}", file);
    let chunk_size = match content_lenght {
        Some(x) => x as usize / 99,
        None => 1024 as usize, // default chunk size
    };

    let mut writer = BufWriter::new(File::create(&file)?);

    loop {
        let mut buffer = vec![0; chunk_size];
        let bcount = resp.read(&mut buffer[..])?;
        buffer.truncate(bcount);
        if !buffer.is_empty() {
            writer.write_all(buffer.as_slice())?;
        } else {
            break;
        }
    }

    Ok(())
}

pub fn get_download_folder(pd_title: &str) -> Result<String> {
    // It might be better to make it a hash of the title
    let download_fold = format!("{}/{}", DL_DIR.to_str().unwrap(), pd_title);

    // Create the folder
    DirBuilder::new().recursive(true).create(&download_fold)?;
    Ok(download_fold)
}

// TODO: Refactor
pub fn get_episode(ep: &mut EpisodeWidgetQuery, download_folder: &str) -> Result<()> {
    // Check if its alrdy downloaded
    if ep.local_uri().is_some() {
        if Path::new(ep.local_uri().unwrap()).exists() {
            return Ok(());
        }

        // If the path is not valid, then set it to None.
        ep.set_local_uri(None);
        ep.save()?;
    };

    let res = download_into(download_folder, ep.title(), ep.uri().unwrap());

    if let Ok(path) = res {
        // If download succedes set episode local_uri to dlpath.
        ep.set_local_uri(Some(&path));

        let size = fs::metadata(path);
        if let Ok(s) = size {
            ep.set_length(Some(s.len() as i32))
        };
        ep.save()?;
        Ok(())
    } else {
        error!("Something whent wrong while downloading.");
        Err(res.unwrap_err())
    }
}

pub fn cache_image(pd: &PodcastCoverQuery) -> Option<String> {
    let url = pd.image_uri()?.to_owned();
    if url == "" {
        return None;
    }

    let download_fold = format!(
        "{}{}",
        HAMMOND_CACHE.to_str().unwrap(),
        pd.title().to_owned()
    );

    // Hacky way
    // TODO: make it so it returns the first cover.* file encountered.
    // Use glob instead
    let png = format!("{}/cover.png", download_fold);
    let jpg = format!("{}/cover.jpg", download_fold);
    let jpe = format!("{}/cover.jpe", download_fold);
    let jpeg = format!("{}/cover.jpeg", download_fold);
    if Path::new(&png).exists() {
        return Some(png);
    } else if Path::new(&jpe).exists() {
        return Some(jpe);
    } else if Path::new(&jpg).exists() {
        return Some(jpg);
    } else if Path::new(&jpeg).exists() {
        return Some(jpeg);
    };

    DirBuilder::new()
        .recursive(true)
        .create(&download_fold)
        .unwrap();

    let dlpath = download_into(&download_fold, "cover", &url);
    if let Ok(path) = dlpath {
        info!("Cached img into: {}", &path);
        Some(path)
    } else {
        error!("Failed to get feed image.");
        error!("Error: {}", dlpath.unwrap_err());
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hammond_data::Source;
    use hammond_data::feed::index;
    use hammond_data::dbqueries;
    use diesel::associations::Identifiable;

    use std::fs;

    #[test]
    fn test_get_dl_folder() {
        let foo_ = format!("{}/{}", DL_DIR.to_str().unwrap(), "foo");
        assert_eq!(get_download_folder("foo").unwrap(), foo_);
        let _ = fs::remove_dir_all(foo_);
    }

    #[test]
    fn test_cache_image() {
        let url = "http://www.newrustacean.com/feed.xml";

        // Create and index a source
        let source = Source::from_url(url).unwrap();
        // Copy it's id
        let sid = source.id().clone();

        // Convert Source it into a Feed and index it
        let feed = source.into_feed().unwrap();
        index(vec![feed]);

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid).unwrap().into();

        let img_path = cache_image(&pd);
        let foo_ = format!(
            "{}{}/cover.png",
            HAMMOND_CACHE.to_str().unwrap(),
            pd.title()
        );
        assert_eq!(img_path, Some(foo_));
    }
}
