use rayon::prelude::*;

use errors::*;
use dbqueries;
use index_feed::Database;
use models::Episode;
use chrono::prelude::*;

use std::path::Path;
use std::fs;

// TODO: Write unit test.
fn download_checker(db: &Database) -> Result<()> {
    let mut episodes = {
        let tempdb = db.lock().unwrap();
        dbqueries::get_downloaded_episodes(&tempdb)?
    };

    episodes.par_iter_mut().for_each(|ep| {
        if !Path::new(ep.local_uri().unwrap()).exists() {
            ep.set_local_uri(None);
            let res = ep.save(&db.clone());
            if let Err(err) = res {
                error!("Error while trying to update episode: {:#?}", ep);
                error!("Error: {}", err);
            };
        }
    });

    Ok(())
}

// TODO: Write unit test.
fn watched_cleaner(db: &Database) -> Result<()> {
    let mut episodes = {
        let tempdb = db.lock().unwrap();
        dbqueries::get_watched_episodes(&tempdb)?
    };

    let now_utc = Utc::now().timestamp() as i32;
    episodes.par_iter_mut().for_each(|mut ep| {
        if ep.local_uri().is_some() && ep.watched().is_some() {
            let watched = ep.watched().unwrap().clone();
            // TODO: expose a config and a user set option.
            let limit = watched + 172_800; // add 2days in seconds
            if now_utc > limit {
                let e = delete_local_content(&db.clone(), &mut ep);
                if let Err(err) = e {
                    error!("Error while trying to delete file: {:?}", ep.local_uri());
                    error!("Error: {}", err);
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

pub fn set_watched(db: &Database, ep: &mut Episode) -> Result<()> {
    let epoch = Utc::now().timestamp() as i32;
    ep.set_watched(Some(epoch));
    ep.save(db)?;
    Ok(())
}

pub fn run(db: &Database) -> Result<()> {
    download_checker(db)?;
    watched_cleaner(db)?;
    Ok(())
}
