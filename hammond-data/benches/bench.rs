#[macro_use]
extern crate criterion;
use criterion::Criterion;

// extern crate futures;
extern crate hammond_data;
extern crate hyper;
extern crate hyper_tls;
extern crate rand;
extern crate tokio_core;
// extern crate rayon;
extern crate rss;

// use rayon::prelude::*;

// use futures::future::*;
use tokio_core::reactor::Core;

use hammond_data::database::truncate_db;
use hammond_data::pipeline;
use hammond_data::FeedBuilder;
use hammond_data::Source;
// use hammond_data::errors::*;

use std::io::BufReader;

// RSS feeds
const INTERCEPTED: &[u8] = include_bytes!("../tests/feeds/2018-01-20-Intercepted.xml");
const INTERCEPTED_URL: &str = "https://web.archive.org/web/20180120083840if_/https://feeds.\
                               feedburner.com/InterceptedWithJeremyScahill";

const UNPLUGGED: &[u8] = include_bytes!("../tests/feeds/2018-01-20-LinuxUnplugged.xml");
const UNPLUGGED_URL: &str =
    "https://web.archive.org/web/20180120110314if_/https://feeds.feedburner.com/linuxunplugged";

const TIPOFF: &[u8] = include_bytes!("../tests/feeds/2018-01-20-TheTipOff.xml");
const TIPOFF_URL: &str =
    "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff";

// This feed has HUGE descripion and summary fields which can be very
// very expensive to parse.
const CODE: &[u8] = include_bytes!("../tests/feeds/2018-01-20-GreaterThanCode.xml");
const CODE_URL: &str =
    "https://web.archive.org/web/20180120104741if_/https://www.greaterthancode.com/feed/podcast";

// Relative small feed
const STARS: &[u8] = include_bytes!("../tests/feeds/2018-01-20-StealTheStars.xml");
const STARS_URL: &str =
    "https://web.archive.org/web/20180120104957if_/https://rss.art19.com/steal-the-stars";

static FEEDS: &[(&[u8], &str)] = &[
    (INTERCEPTED, INTERCEPTED_URL),
    (UNPLUGGED, UNPLUGGED_URL),
    (TIPOFF, TIPOFF_URL),
    (CODE, CODE_URL),
    (STARS, STARS_URL),
];

// This is broken and I don't know why.
fn bench_pipeline(c: &mut Criterion) {
    truncate_db().unwrap();
    FEEDS.iter().for_each(|&(_, url)| {
        Source::from_url(url).unwrap();
    });

    c.bench_function("pipline", move |b| {
        b.iter(|| {
            let sources = hammond_data::dbqueries::get_sources().unwrap();
            pipeline::run(sources, true).unwrap();
        })
    });
    truncate_db().unwrap();
}

fn bench_index_large_feed(c: &mut Criterion) {
    truncate_db().unwrap();
    let url = "https://www.greaterthancode.com/feed/podcast";
    let mut core = Core::new().unwrap();

    c.bench_function("index_large_feed", move |b| {
        b.iter(|| {
            let s = Source::from_url(url).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(CODE)).unwrap();
            let feed = FeedBuilder::default()
                .channel(chan)
                .source_id(s.id())
                .build()
                .unwrap();
            core.run(feed.index()).unwrap();
        })
    });
    truncate_db().unwrap();
}

fn bench_index_small_feed(c: &mut Criterion) {
    truncate_db().unwrap();
    let url = "https://rss.art19.com/steal-the-stars";
    let mut core = Core::new().unwrap();

    c.bench_function("index_small_feed", move |b| {
        b.iter(|| {
            let s = Source::from_url(url).unwrap();
            // parse it into a channel
            let chan = rss::Channel::read_from(BufReader::new(STARS)).unwrap();
            let feed = FeedBuilder::default()
                .channel(chan)
                .source_id(s.id())
                .build()
                .unwrap();
            core.run(feed.index()).unwrap();
        })
    });
    truncate_db().unwrap();
}

criterion_group!(
    benches,
    bench_pipeline,
    bench_index_large_feed,
    bench_index_small_feed
);
criterion_main!(benches);
