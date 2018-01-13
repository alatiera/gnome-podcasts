#![feature(test)]

extern crate diesel;
extern crate futures;
extern crate hammond_data;
extern crate hyper;
extern crate hyper_tls;
extern crate rand;
extern crate rayon;
extern crate rss;
extern crate tempdir;
extern crate test;
extern crate tokio_core;

use rayon::prelude::*;

use test::Bencher;

use tokio_core::reactor::Core;
use hyper::Client;
use hyper_tls::HttpsConnector;
use futures::future::*;

use hammond_data::Source;
use hammond_data::feed::*;
use hammond_data::database::truncate_db;

use std::io::BufReader;

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

fn index_urls() {
    let feeds: Vec<_> = URLS.par_iter()
        .map(|&(buff, url)| {
            // Create and insert a Source into db
            let s = Source::from_url(url).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(buff)).unwrap();
            Feed::from_channel_source(chan, s.id())
        })
        .collect();

    feeds.par_iter().for_each(|x| index(x));
}

#[bench]
fn bench_index_feeds(b: &mut Bencher) {
    b.iter(|| {
        index_urls();
    });
}

#[bench]
fn bench_index_unchanged_feeds(b: &mut Bencher) {
    // Index first so it will only bench the comparison test case.
    index_urls();

    b.iter(|| {
        for _ in 0..10 {
            index_urls();
        }
    });
}

#[bench]
fn bench_get_normal_feeds(b: &mut Bencher) {
    truncate_db().unwrap();

    b.iter(|| {
        URLS.iter().for_each(|&(_, url)| {
            let mut s = Source::from_url(url).unwrap();
            s.into_feed(true).unwrap();
        })
    });
}

#[bench]
fn bench_get_future_feeds(b: &mut Bencher) {
    truncate_db().unwrap();

    b.iter(|| {
        let mut core = Core::new().unwrap();
        let mut handle = core.handle();
        let mut client = Client::configure()
            .connector(HttpsConnector::new(4, &handle).unwrap())
            .build(&handle);

        let mut foo: Vec<_>;

        URLS.iter().for_each(|&(_, url)| {
            let mut s = Source::from_url(url).unwrap();
            let future = s.into_fututre_feed(&mut client, true);
            foo.push(future);
        });

        let work = join_all(foo);
        core.run(work).unwrap();
    });
}
