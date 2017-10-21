use reqwest;
use hyper::header::*;
use diesel::prelude::*;

use std::fs::{rename, DirBuilder, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::thread;

use errors::*;
use hammond_data::dbqueries;
use hammond_data::models::Episode;
use hammond_data::{DL_DIR, HAMMOND_CACHE};

// Adapted from https://github.com/mattgathu/rget .
// I never wanted to write a custom downloader.
// Sorry to those who will have to work with that code.
// Would much rather use a crate,
// or bindings for a lib like youtube-dl(python),
// But cant seem to find one.
pub fn download_to(target: &str, url: &str) -> Result<()> {
    info!("GET request to: {}", url);
    let mut resp = reqwest::get(url)?;
    info!("Status Resp: {}", resp.status());

    if resp.status().is_success() {
        let headers = resp.headers().clone();

        let ct_len = headers.get::<ContentLength>().map(|ct_len| **ct_len);
        let ct_type = headers.get::<ContentType>().unwrap();
        ct_len.map(|x| info!("File Lenght: {}", x));
        info!("Content Type: {:?}", ct_type);

        info!("Save destination: {}", target);

        let chunk_size = match ct_len {
            Some(x) => x as usize / 99,
            None => 1024 as usize, // default chunk size
        };

        let out_file = format!("{}.part", target);
        let mut writer = BufWriter::new(File::create(&out_file)?);

        loop {
            let mut buffer = vec![0; chunk_size];
            let bcount = resp.read(&mut buffer[..]).unwrap();
            buffer.truncate(bcount);
            if !buffer.is_empty() {
                writer.write_all(buffer.as_slice()).unwrap();
            } else {
                break;
            }
        }
        rename(out_file, target)?;
    }
    Ok(())
}

// Initial messy prototype, queries load alot of not needed stuff.
// TODO: Refactor
pub fn latest_dl(connection: &SqliteConnection, limit: u32) -> Result<()> {
    let pds = dbqueries::get_podcasts(connection)?;

    let _: Vec<_> = pds.iter()
        // This could be for_each instead of map.
        .map(|x| -> Result<()> {
            let mut eps = if limit == 0 {
                dbqueries::get_pd_episodes(connection, x)?
            } else {
                dbqueries::get_pd_episodes_limit(connection, x, limit)?
            };

            let dl_fold = get_dl_folder(x.title())?;

            // Download the episodes
            let _ :Vec<_> = eps.iter_mut()
                .map(|ep| -> Result<()> {
                    // TODO: handle Result here and replace map with for_each
                    get_episode(connection, ep, &dl_fold)
                })
                .collect();

            Ok(())
        })
        .collect();

    Ok(())
}

// TODO: Right unit test
pub fn get_dl_folder(pd_title: &str) -> Result<String> {
    // It might be better to make it a hash of the title
    let dl_fold = format!("{}/{}", DL_DIR.to_str().unwrap(), pd_title);

    // Create the folder
    // TODO: handle the unwrap properly
    DirBuilder::new().recursive(true).create(&dl_fold)?;
    Ok(dl_fold)
}

pub fn get_episode(connection: &SqliteConnection, ep: &mut Episode, dl_folder: &str) -> Result<()> {
    // Check if its alrdy downloaded
    if ep.local_uri().is_some() {
        if Path::new(ep.local_uri().unwrap()).exists() {
            return Ok(());
        }
        ep.set_local_uri(None);
        ep.save_changes::<Episode>(connection)?;
    };

    // Unreliable and hacky way to extract the file extension from the url.
    let ext = ep.uri().split('.').last().unwrap().to_owned();

    // Construct the download path.
    // TODO: Check if its a valid path
    let dlpath = format!("{}/{}.{}", dl_folder, ep.title().unwrap().to_owned(), ext);
    // info!("Downloading {:?} into: {}", y.title(), dlpath);

    // If download succedes set episode local_uri to dlpath.
    // This should be at the end after download is finished,
    // but its handy atm while the gtk client doesnt yet have custom signals.
    ep.set_local_uri(Some(&dlpath));
    ep.save_changes::<Episode>(connection)?;

    let uri = ep.uri().to_owned();

    // This would not be needed in general but I want to be able to call
    // this function from the gtk client.
    // should get removed probably once custom callbacks are implemented.
    thread::spawn(move || {
        let res = download_to(&dlpath, uri.as_str());
        if let Err(err) = res {
            error!("Something whent wrong while downloading.");
            error!("Error: {}", err);
        } else {
            info!("Download of {} finished.", uri);
        }
    });
    Ok(())
}

// pub fn cache_image(pd: &Podcast) -> Option<String> {
// TODO: Right unit test
// TODO: Refactor
pub fn cache_image(title: &str, image_uri: Option<&str>) -> Option<String> {
    if let Some(url) = image_uri {
        if url == "" {
            return None;
        }

        let ext = url.split('.').last().unwrap();

        let dl_fold = format!("{}{}", HAMMOND_CACHE.to_str().unwrap(), title);
        DirBuilder::new().recursive(true).create(&dl_fold).unwrap();

        let dlpath = format!("{}/{}.{}", dl_fold, title, ext);

        if Path::new(&dlpath).exists() {
            return Some(dlpath);
        }

        if let Err(err) = download_to(&dlpath, url) {
            error!("Failed to get feed image.");
            error!("Error: {}", err);
            return None;
        };

        info!("Cached img into: {}", dlpath);
        return Some(dlpath);
    }
    None
}
