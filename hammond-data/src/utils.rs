use rayon::prelude::*;
use chrono::prelude::*;

use url::{Position, Url};

use errors::*;
use dbqueries;
use models::queryables::Episode;

use std::path::Path;
use std::fs;

// TODO: Write unit test.
fn download_checker() -> Result<()> {
    let episodes = dbqueries::get_downloaded_episodes()?;

    episodes.into_par_iter().for_each(|mut ep| {
        if !Path::new(ep.local_uri().unwrap()).exists() {
            ep.set_local_uri(None);
            let res = ep.save();
            if let Err(err) = res {
                error!("Error while trying to update episode: {:#?}", ep);
                error!("Error: {}", err);
            };
        }
    });

    Ok(())
}

// TODO: Write unit test.
fn played_cleaner() -> Result<()> {
    let episodes = dbqueries::get_played_episodes()?;

    let now_utc = Utc::now().timestamp() as i32;
    episodes.into_par_iter().for_each(|mut ep| {
        if ep.local_uri().is_some() && ep.played().is_some() {
            let played = ep.played().unwrap();
            // TODO: expose a config and a user set option.
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

// TODO: Write unit test.
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

pub fn checkup() -> Result<()> {
    download_checker()?;
    played_cleaner()?;
    Ok(())
}

pub fn url_cleaner(s: &str) -> String {
    // Copied from the cookbook.
    // https://rust-lang-nursery.github.io/rust-cookbook/net.html
    // #remove-fragment-identifiers-and-query-pairs-from-a-url
    match Url::parse(s) {
        Ok(parsed) => parsed[..Position::AfterPath].to_owned(),
        _ => s.trim().to_owned(),
    }
}
