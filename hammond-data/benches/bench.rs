#![feature(test)]

extern crate futures;
extern crate hammond_data;
extern crate hyper;
extern crate hyper_tls;
extern crate rand;
extern crate tokio_core;
// extern crate rayon;
extern crate rss;
extern crate test;

// use rayon::prelude::*;

use futures::future::*;
use test::Bencher;
use tokio_core::reactor::Core;

use hammond_data::Source;
use hammond_data::database::truncate_db;
use hammond_data::errors::*;
use hammond_data::feed::*;

use std::io::BufReader;

// Big rss feed
const PCPER: &[u8] = include_bytes!("feeds/pcpermp3.xml");
const UNPLUGGED: &[u8] = include_bytes!("feeds/linuxunplugged.xml");
const RADIO: &[u8] = include_bytes!("feeds/coderradiomp3.xml");
const SNAP: &[u8] = include_bytes!("feeds/techsnapmp3.xml");
const LAS: &[u8] = include_bytes!("feeds/TheLinuxActionShow.xml");

// This feed has HUGE descripion and summary fields which can be very
// very expensive to parse.
const CODE: &[u8] = include_bytes!("feeds/GreaterThanCode.xml");
// Relative small feed
const STARS: &[u8] = include_bytes!("feeds/StealTheStars.xml");

static URLS: &[(&[u8], &str)] = &[
    (PCPER, "https://www.pcper.com/rss/podcasts-mp3.rss"),
    (UNPLUGGED, "http://feeds.feedburner.com/linuxunplugged"),
    (RADIO, "https://feeds.feedburner.com/coderradiomp3"),
    (SNAP, "https://feeds.feedburner.com/techsnapmp3"),
    (LAS, "https://feeds2.feedburner.com/TheLinuxActionShow"),
];

fn index_urls() {
    let feeds: Vec<_> = URLS.iter()
        .map(|&(buff, url)| {
            // Create and insert a Source into db
            let s = Source::from_url(url).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(buff)).unwrap();
            Feed::from_channel_source(chan, s.id())
        })
        .collect();

    feeds.iter().for_each(|x| x.index().unwrap());
}

fn index_urls_async() -> Vec<Box<Future<Item = (), Error = Error>>> {
    let feeds: Vec<_> = URLS.iter()
        .map(|&(buff, url)| {
            // Create and insert a Source into db
            let s = Source::from_url(url).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(buff)).unwrap();
            Feed::from_channel_source(chan, s.id())
        })
        .collect();

    feeds.into_iter().map(|feed| feed.index_async()).collect()
}

#[bench]
fn bench_index_feeds(b: &mut Bencher) {
    truncate_db().unwrap();

    b.iter(|| {
        index_urls();
    });
}

#[bench]
fn bench_index_feeds_async(b: &mut Bencher) {
    truncate_db().unwrap();
    let mut core = Core::new().unwrap();

    b.iter(|| {
        let list = index_urls_async();
        let _foo = core.run(select_all(list));
    });
}

#[bench]
fn bench_index_unchanged_feeds(b: &mut Bencher) {
    truncate_db().unwrap();
    // Index first so it will only bench the comparison test case.
    index_urls();

    b.iter(|| {
        for _ in 0..10 {
            index_urls();
        }
    });
}

#[bench]
fn bench_get_future_feeds(b: &mut Bencher) {
    truncate_db().unwrap();
    URLS.iter().for_each(|&(_, url)| {
        Source::from_url(url).unwrap();
    });

    b.iter(|| {
        let sources = hammond_data::dbqueries::get_sources().unwrap();
        hammond_data::pipeline::pipeline(sources, false).unwrap();
    })
}

#[bench]
fn bench_index_greater_than_code(b: &mut Bencher) {
    truncate_db().unwrap();
    let url = "https://www.greaterthancode.com/feed/podcast";

    b.iter(|| {
        let s = Source::from_url(url).unwrap();
        // parse it into a channel
        let chan = rss::Channel::read_from(BufReader::new(CODE)).unwrap();
        let feed = Feed::from_channel_source(chan, s.id());
        feed.index().unwrap();
    })
}

#[bench]
fn bench_index_steal_the_stars(b: &mut Bencher) {
    truncate_db().unwrap();
    let url = "https://rss.art19.com/steal-the-stars";

    b.iter(|| {
        let s = Source::from_url(url).unwrap();
        // parse it into a channel
        let chan = rss::Channel::read_from(BufReader::new(STARS)).unwrap();
        let feed = Feed::from_channel_source(chan, s.id());
        feed.index().unwrap();
    })
}
