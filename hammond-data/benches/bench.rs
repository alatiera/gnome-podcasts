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
use hammond_data::index_feed::{complete_index, insert_return_source};

use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::fs;

struct TempDB(tempdir::TempDir, PathBuf, SqliteConnection);

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

#[bench]
fn bench_index_test_files(b: &mut Bencher) {
    let TempDB(_tmp_dir, _db_path, db) = get_temp_db();
    // complete_index runs in parallel so it requires a mutex as argument.
    let m = Arc::new(Mutex::new(db));

    // include them in the binary to avoid loading from disk making file open syscalls.
    let pcper = include_bytes!("feeds/pcpermp3.xml");
    let unplugged = include_bytes!("feeds/linuxunplugged.xml");
    let radio = include_bytes!("feeds/coderradiomp3.xml");
    let snap = include_bytes!("feeds/techsnapmp3.xml");
    let las = include_bytes!("feeds/TheLinuxActionShow.xml");

    // vec of (&vec<u8>, url) tuples.
    let urls = vec![
        (pcper.as_ref(), "https://www.pcper.com/rss/podcasts-mp3.rss"),
        (
            unplugged.as_ref(),
            "http://feeds.feedburner.com/linuxunplugged",
        ),
        (radio.as_ref(), "https://feeds.feedburner.com/coderradiomp3"),
        (snap.as_ref(), "https://feeds.feedburner.com/techsnapmp3"),
        (
            las.as_ref(),
            "https://feeds2.feedburner.com/TheLinuxActionShow",
        ),
    ];

    b.iter(|| {
        urls.par_iter().for_each(|&(buff, url)| {
            // Create and insert a Source into db
            let s = {
                let temp = m.lock().unwrap();
                insert_return_source(&temp, url).unwrap()
            };
            // parse it into a channel
            let chan = rss::Channel::read_from(buff).unwrap();

            // Index the channel
            complete_index(&m, &chan, &s).unwrap();
        });
    });
}
