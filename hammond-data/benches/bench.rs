#![feature(test)]

extern crate diesel;
extern crate hammond_data;
extern crate rand;
extern crate rayon;
extern crate rss;
extern crate tempdir;
extern crate test;

use diesel::prelude::*;
use rayon::prelude::*;

use rand::Rng;
use test::Bencher;

use hammond_data::run_migration_on;
use hammond_data::models::NewSource;
use hammond_data::feed::{index, Feed};
use hammond_data::Database;

use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct TempDB(tempdir::TempDir, PathBuf, SqliteConnection);

// Big rss feed
const PCPER: &[u8] = include_bytes!("feeds/pcpermp3.xml");
const UNPLUGGED: &[u8] = include_bytes!("feeds/linuxunplugged.xml");
const RADIO: &[u8] = include_bytes!("feeds/coderradiomp3.xml");
const SNAP: &[u8] = include_bytes!("feeds/techsnapmp3.xml");
const LAS: &[u8] = include_bytes!("feeds/TheLinuxActionShow.xml");

static URLS: &[(&[u8], &str)] = &[
    (PCPER, "https://www.pcper.com/rss/podcasts-mp3.rss"),
    (UNPLUGGED, "http://feeds.feedburner.com/linuxunplugged"),
    (RADIO, "https://feeds.feedburner.com/coderradiomp3"),
    (SNAP, "https://feeds.feedburner.com/techsnapmp3"),
    (LAS, "https://feeds2.feedburner.com/TheLinuxActionShow"),
];

/// Create and return a Temporary DB.
/// Will be destroed once the returned variable(s) is dropped.
fn get_temp_db() -> TempDB {
    let mut rng = rand::thread_rng();

    let tmp_dir = tempdir::TempDir::new("hammond_unit_test").unwrap();
    let db_path = tmp_dir
        .path()
        .join(format!("hammonddb_{}.db", rng.gen::<usize>()));

    let db = SqliteConnection::establish(db_path.to_str().unwrap()).unwrap();
    ::run_migration_on(&db).unwrap();

    TempDB(tmp_dir, db_path, db)
}

fn index_urls(m: &Database) {
    URLS.par_iter()
        .map(|&(buff, url)| {
            // Create and insert a Source into db
            let s = NewSource::new_with_uri(url).into_source(m).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(buff)).unwrap();
            Feed::new_from_channel_source(chan, s)
        })
        .for_each(|feed| {
            index(m, &mut [feed]);
        });
}

#[bench]
fn bench_index_feeds(b: &mut Bencher) {
    let TempDB(_tmp_dir, _db_path, db) = get_temp_db();
    let m = Arc::new(Mutex::new(db));

    b.iter(|| {
        index_urls(&Arc::clone(&m));
    });
}

#[bench]
fn bench_index_unchanged_feeds(b: &mut Bencher) {
    let TempDB(_tmp_dir, _db_path, db) = get_temp_db();
    let m = Arc::new(Mutex::new(db));

    // Index first so it will only bench the comparison test case.
    index_urls(&Arc::clone(&m));

    b.iter(|| {
        for _ in 0..10 {
            index_urls(&Arc::clone(&m));
        }
    });
}
