use reqwest;
use hyper::header::*;
// use mime::Mime;

use std::fs::{rename, DirBuilder, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
// use std::str::FromStr;

use errors::*;
use hammond_data::index_feed::Database;
use hammond_data::models::Episode;
use hammond_data::{DL_DIR, HAMMOND_CACHE};

// Adapted from https://github.com/mattgathu/rget .
// I never wanted to write a custom downloader.
// Sorry to those who will have to work with that code.
// Would much rather use a crate,
// or bindings for a lib like youtube-dl(python),
// But cant seem to find one.
pub fn download_to(dir: &str, filename: &str, url: &str) -> Result<String> {
    info!("GET request to: {}", url);
    let client = reqwest::Client::builder().referer(false).build()?;
    let mut resp = client.get(url).send()?;
    info!("Status Resp: {}", resp.status());

    if resp.status().is_success() {
        let headers = resp.headers().clone();

        let ct_len = headers.get::<ContentLength>().map(|ct_len| **ct_len);
        let ct_type = headers.get::<ContentType>();
        ct_len.map(|x| info!("File Lenght: {}", x));
        ct_type.map(|x| info!("Content Type: {}", x));

        let target = format!("{}/{}", dir, filename);
        // let target = format!("{}{}",dir, filename, ext);
        return save_io(&target, &mut resp, ct_len);
    }
    // Ok(String::from(""))
    panic!("foo");
}

fn save_io(
    target: &str,
    resp: &mut reqwest::Response,
    content_lenght: Option<u64>,
) -> Result<String> {
    info!("Downloading into: {}", target);
    let chunk_size = match content_lenght {
        Some(x) => x as usize / 99,
        None => 1024 as usize, // default chunk size
    };

    let out_file = format!("{}.part", target);
    let mut writer = BufWriter::new(File::create(&out_file)?);

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
    rename(out_file, target)?;
    info!("Downloading of {} completed succesfully.", target);
    Ok(target.to_string())
}

pub fn get_download_folder(pd_title: &str) -> Result<String> {
    // It might be better to make it a hash of the title
    let download_fold = format!("{}/{}", DL_DIR.to_str().unwrap(), pd_title);

    // Create the folder
    DirBuilder::new().recursive(true).create(&download_fold)?;
    Ok(download_fold)
}

// TODO: Refactor
pub fn get_episode(connection: &Database, ep: &mut Episode, download_folder: &str) -> Result<()> {
    // Check if its alrdy downloaded
    if ep.local_uri().is_some() {
        if Path::new(ep.local_uri().unwrap()).exists() {
            return Ok(());
        }

        ep.set_local_uri(None);
        ep.save(connection)?;
    };

    // FIXME: Unreliable and hacky way to extract the file extension from the url.
    // https://gitlab.gnome.org/alatiera/Hammond/issues/5
    let ext = ep.uri().split('.').last().unwrap().to_owned();

    // Construct the download path.
    // TODO: Check if its a valid path
    let file_name = format!("/{}.{}", ep.title().unwrap().to_owned(), ext);

    let uri = ep.uri().to_owned();
    let res = download_to(download_folder, &file_name, uri.as_str());

    if res.is_ok() {
        // If download succedes set episode local_uri to dlpath.
        let dlpath = res.unwrap();
        ep.set_local_uri(Some(&dlpath));
        ep.save(connection)?;
        Ok(())
    } else {
        error!("Something whent wrong while downloading.");
        Err(res.unwrap_err())
    }
}

// pub fn cache_image(pd: &Podcast) -> Option<String> {
// TODO: Right unit test
// TODO: Refactor
pub fn cache_image(title: &str, image_uri: Option<&str>) -> Option<String> {
    if let Some(url) = image_uri {
        if url == "" {
            return None;
        }

        // FIXME: https://gitlab.gnome.org/alatiera/Hammond/issues/5
        let ext = url.split('.').last().unwrap();

        let download_fold = format!("{}{}", HAMMOND_CACHE.to_str().unwrap(), title);
        DirBuilder::new()
            .recursive(true)
            .create(&download_fold)
            .unwrap();
        let file_name = format!("cover.{}", ext);

        // This will need rework once the #5 is completed.
        let dlpath = format!("{}/{}", download_fold, file_name);

        if Path::new(&dlpath).exists() {
            return Some(dlpath);
        }

        if let Err(err) = download_to(&download_fold, &file_name, url) {
            error!("Failed to get feed image.");
            error!("Error: {}", err);
            return None;
        };

        info!("Cached img into: {}", dlpath);
        return Some(dlpath);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use hammond_data::{DL_DIR, HAMMOND_CACHE};

    #[test]
    fn test_get_dl_folder() {
        let foo_ = format!("{}/{}", DL_DIR.to_str().unwrap(), "foo");
        assert_eq!(get_download_folder("foo").unwrap(), foo_);
    }

    #[test]
    fn test_cache_image() {
        let img_path =
            cache_image("New Rustacean", Some("http://newrustacean.com/podcast.png")).unwrap();
        let foo_ = format!(
            "{}{}/cover.png",
            HAMMOND_CACHE.to_str().unwrap(),
            "New Rustacean"
        );
        assert_eq!(img_path, foo_);
    }
}
