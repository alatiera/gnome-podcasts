#[macro_use]
extern crate criterion;
use criterion::Criterion;

extern crate futures;
extern crate hammond_data;
extern crate hyper;
extern crate hyper_tls;
extern crate rand;
extern crate tokio_core;
// extern crate rayon;
extern crate rss;

// use rayon::prelude::*;

use futures::future::*;
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

fn index_urls() -> Vec<Box<Future<Item = (), Error = Error>>> {
    let feeds: Vec<_> = URLS.iter()
        .map(|&(buff, url)| {
            // Create and insert a Source into db
            let s = Source::from_url(url).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(buff)).unwrap();

            FeedBuilder::default()
                .channel(chan)
                .source_id(s.id())
                .build()
                .unwrap()
        })
        .collect();

    feeds.into_iter().map(|feed| feed.index()).collect()
}

fn bench_index_feeds(c: &mut Criterion) {
    truncate_db().unwrap();
    let mut core = Core::new().unwrap();

    c.bench_function("index_feeds", |b| {
        b.iter(|| {
            let list = index_urls();
            let _foo = core.run(join_all(list));
        })
    });
    truncate_db().unwrap();
}

fn bench_index_unchanged_feeds(c: &mut Criterion) {
    truncate_db().unwrap();
    let mut core = Core::new().unwrap();
    // Index first so it will only bench the comparison test case.
    let list = index_urls();
    let _foo = core.run(join_all(list));

    c.bench_function("index_5_unchanged", |b| {
        b.iter(|| {
            for _ in 0..5 {
                let list = index_urls();
                let _foo = core.run(join_all(list));
            }
        })
    });
    truncate_db().unwrap();
}

// This is broken and I don't know why.
fn bench_pipeline(c: &mut Criterion) {
    truncate_db().unwrap();
    URLS.iter().for_each(|&(_, url)| {
        Source::from_url(url).unwrap();
    });

    c.bench_function("pipline", |b| {
        b.iter(|| {
            let sources = hammond_data::dbqueries::get_sources().unwrap();
            hammond_data::pipeline::pipeline(sources, true).unwrap();
        })
    });
    truncate_db().unwrap();
}

fn bench_index_large_feed(c: &mut Criterion) {
    truncate_db().unwrap();
    let url = "https://www.greaterthancode.com/feed/podcast";
    let mut core = Core::new().unwrap();

    c.bench_function("index_large_feed", |b| {
        b.iter(|| {
            let s = Source::from_url(url).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(CODE)).unwrap();
            let feed = FeedBuilder::default()
                .channel(chan)
                .source_id(s.id())
                .build()
                .unwrap();
            let _foo = core.run(feed.index()).unwrap();
        })
    });
    truncate_db().unwrap();
}

fn bench_index_small_feed(c: &mut Criterion) {
    truncate_db().unwrap();
    let url = "https://rss.art19.com/steal-the-stars";
    let mut core = Core::new().unwrap();

    c.bench_function("index_small_feed", |b| {
        b.iter(|| {
            let s = Source::from_url(url).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(STARS)).unwrap();
            let feed = FeedBuilder::default()
                .channel(chan)
                .source_id(s.id())
                .build()
                .unwrap();
            let _foo = core.run(feed.index()).unwrap();
        })
    });
    truncate_db().unwrap();
}

criterion_group!(
    benches,
    bench_index_feeds,
    bench_index_unchanged_feeds,
    bench_pipeline,
    bench_index_large_feed,
    bench_index_small_feed
);
criterion_main!(benches);
