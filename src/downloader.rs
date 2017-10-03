use reqwest;
use hyper::header::*;
use diesel::prelude::*;

use std::fs::{DirBuilder, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use errors::*;
use dbqueries;

// Adapted from https://github.com/mattgathu/rget .
/// I never wanted to write a custom downloader.
/// Sorry to those who will have to work with that code.
/// Would much rather use a crate, 
/// or bindings for a lib like youtube-dl(python),
/// But cant seem to find one.
pub fn download_to(target: &str, url: &str) -> Result<()> {
    let mut resp = reqwest::get(url)?;
    info!("GET request to: {}", url);

    if resp.status().is_success() {
        let headers = resp.headers().clone();

        let ct_len = headers.get::<ContentLength>().map(|ct_len| **ct_len);
        let ct_type = headers.get::<ContentType>().unwrap();
        ct_len.map(|x| info!("File Lenght: {}", x));
        info!("Content Type: {:?}", ct_type);

        // FIXME
        // let out_file = target.to_owned() + "/bar.mp3";
        info!("Save destination: {}", target);

        let chunk_size = match ct_len {
            Some(x) => x as usize / 99,
            None => 1024 as usize, // default chunk size
        };

        let mut writer = BufWriter::new(File::create(target)?);

        loop {
            let mut buffer = vec![0; chunk_size];
            let bcount = resp.read(&mut buffer[..]).unwrap();
            buffer.truncate(bcount);
            if !buffer.is_empty() {
                writer.write(buffer.as_slice()).unwrap();
            } else {
                break;
            }
        }
    }
    Ok(())
}

// Initial messy prototype, queries load alot of not needed stuff.
pub fn latest_dl(connection: &SqliteConnection, limit: u32) -> Result<()> {
    use models::Episode;

    let pds = dbqueries::get_podcasts(connection)?;

    pds.iter()
        // TODO when for_each reaches stable:
        // Remove all the ugly folds(_) and replace map() with for_each().
        .map(|x| -> Result<()> {
            let mut eps;
            if limit == 0 {
                eps = dbqueries::get_pd_episodes(connection, &x)?;
            } else {
                eps = dbqueries::get_pd_episodes_limit(connection, &x, limit)?;
            }

            // It might be better to make it a hash of the title
            let dl_fold = format!("{}/{}", ::DL_DIR.to_str().unwrap(), x.title());

            // Create the folder
            DirBuilder::new().recursive(true).create(&dl_fold).unwrap();

            // Download the episodes
            eps.iter_mut()
                .map(|y| -> Result<()> {
                    // Check if its alrdy downloaded
                    if let Some(foo) = y.local_uri().clone(){
                        if Path::new(foo).exists() {
                            return Ok(());
                        }
                        y.save_changes::<Episode>(connection)?;
                        ()
                    };

                    // Unreliable and hacky way to extract the file extension from the url.
                    let ext = y.uri().split(".").last().unwrap().to_owned();

                    // Construct the download path.
                    let dlpath = format!("{}/{}.{}", dl_fold, y.title().unwrap().to_owned(), ext);
                    info!("Downloading {:?} into: {}", y.title(), dlpath);
                    // TODO: implement .part files
                    download_to(&dlpath, y.uri())?;

                    // If download succedes set episode local_uri to dlpath.
                    y.set_local_uri(Some(&dlpath));
                    y.save_changes::<Episode>(connection)?;
                    Ok(())
                })
                .fold((), |(), _| ());

            Ok(())
        })
        .fold((), |(), _| ());

    Ok(())
}
