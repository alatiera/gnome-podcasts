use reqwest;
use hyper::header::*;
use diesel::prelude::*;

use std::fs::{File, DirBuilder};
use std::io::{BufWriter, Read, Write};

use errors::*;
use dbqueries;

// Adapted from https://github.com/mattgathu/rget .
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

        // FIXME: not running
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
pub fn latest_dl(connection: &SqliteConnection) -> Result<()> {
    let pds = dbqueries::get_podcasts(connection)?;
 
    pds.iter()
        .map(|x| -> Result<()> {
            let eps = dbqueries::get_pd_episodes(connection, &x)?;

            // It might be better to make it a hash of the title
            let dl_fold = format!("{}/{}", ::DL_DIR.to_str().unwrap(), x.title());

            // Create the folder
            DirBuilder::new().recursive(true).create(&dl_fold).unwrap();

            // Download the episodes
            eps.iter()
                .map(|y| -> Result<()> {
                    let ext = y.uri().split(".").last().unwrap();
                    let dlpath = format!("{}/{}.{}", dl_fold, y.title().unwrap(), ext);
                    info!("Downloading {:?} into: {}", y.title(), dlpath);
                    download_to(&dlpath, y.uri())?;
                    Ok(())
                })
                .fold((), |(), _| ());

            Ok(())
        })
        .fold((), |(), _| ());

    Ok(())
}
