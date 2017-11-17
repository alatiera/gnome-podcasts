use rayon::prelude::*;

use errors::*;
use dbqueries;
use Database;
use models::Episode;
use chrono::prelude::*;

use std::path::Path;
use std::fs;
use std::sync::Arc;

// TODO: Write unit test.
fn download_checker(db: &Database) -> Result<()> {
    let mut episodes = {
        let tempdb = db.lock().unwrap();
        dbqueries::get_downloaded_episodes(&tempdb)?
    };

    episodes.par_iter_mut().for_each(|ep| {
        if !Path::new(ep.local_uri().unwrap()).exists() {
            ep.set_local_uri(None);
            let res = ep.save(&Arc::clone(db));
            if let Err(err) = res {
                error!("Error while trying to update episode: {:#?}", ep);
                error!("Error: {}", err);
            };
        }
    });

    Ok(())
}

// TODO: Write unit test.
fn played_cleaner(db: &Database) -> Result<()> {
    let mut episodes = {
        let tempdb = db.lock().unwrap();
        dbqueries::get_played_episodes(&tempdb)?
    };

    let now_utc = Utc::now().timestamp() as i32;
    episodes.par_iter_mut().for_each(|mut ep| {
        if ep.local_uri().is_some() && ep.played().is_some() {
            let played = ep.played().unwrap();
            // TODO: expose a config and a user set option.
            let limit = played + 172_800; // add 2days in seconds
            if now_utc > limit {
                let e = delete_local_content(&Arc::clone(db), &mut ep);
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
pub fn delete_local_content(db: &Database, ep: &mut Episode) -> Result<()> {
    if ep.local_uri().is_some() {
        let uri = ep.local_uri().unwrap().to_owned();
        if Path::new(&uri).exists() {
            let res = fs::remove_file(&uri);
            if res.is_ok() {
                ep.set_local_uri(None);
                ep.save(db)?;
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

pub fn set_played_now(db: &Database, ep: &mut Episode) -> Result<()> {
    let epoch = Utc::now().timestamp() as i32;
    ep.set_played(Some(epoch));
    ep.save(db)?;
    Ok(())
}

pub fn checkup(db: &Database) -> Result<()> {
    download_checker(db)?;
    played_cleaner(db)?;
    Ok(())
}
