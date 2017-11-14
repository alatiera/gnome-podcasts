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

    // vec of (path, url) tuples.
    let urls = vec![
        (
            "tests/feeds/Intercepted.xml",
            "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
        ),
        (
            "tests/feeds/LinuxUnplugged.xml",
            "http://feeds.feedburner.com/linuxunplugged",
        ),
        (
            "tests/feeds/TheBreakthrough.xml",
            "http://feeds.feedburner.com/propublica/podcast",
        ),
        (
            "tests/feeds/R4Explanation.xml",
            "https://request-for-explanation.github.io/podcast/rss.xml",
        ),
    ];

    b.iter(|| {
        urls.par_iter().for_each(|&(path, url)| {
            let tempdb = m.lock().unwrap();
            // Create and insert a Source into db
            let s = insert_return_source(&tempdb, url).unwrap();
            drop(tempdb);

            // open the xml file
            let feed = fs::File::open(path).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(feed)).unwrap();

            // Index the channel
            complete_index(&m, &chan, &s).unwrap();
        });
    });
}
