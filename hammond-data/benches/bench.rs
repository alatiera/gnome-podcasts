#![feature(test)]

extern crate futures;
extern crate hammond_data;
extern crate hyper;
extern crate hyper_tls;
extern crate rand;
extern crate rayon;
extern crate rss;
extern crate test;
extern crate tokio_core;

// use rayon::prelude::*;

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

static URLS2: &[&str] = &[
    "https://feeds.feedburner.com/InterceptedWithJeremyScahill",
    "http://www.badvoltage.org/feed/ogg/",
    "https://www.theguardian.com/news/series/the-audio-long-read/podcast.xml",
    "http://feeds.feedburner.com/coderradiomp3",
    "https://rss.art19.com/steal-the-stars",
    "https://feeds.mozilla-podcasts.org/irl",
    "http://economicupdate.libsyn.com/rss",
    "http://feeds.feedburner.com/linuxunplugged",
    "http://ubuntupodcast.org/feed/ogg/",
    "http://www.newrustacean.com/feed.xml",
    "http://feeds.propublica.org/propublica/podcast",
    "https://rss.acast.com/thetipoff",
    "http://feeds.soundcloud.com/users/soundcloud:users:277306156/sounds.rss",
    "http://revolutionspodcast.libsyn.com/rss/",
    "https://www.greaterthancode.com/feed/podcast",
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

    feeds.iter().for_each(|x| index(x));
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
    // Index first so it will only bench the comparison test case.
    truncate_db().unwrap();

    b.iter(|| {
        URLS2.iter().for_each(|url| {
            let mut s = Source::from_url(url).unwrap();
            let _feed = s.into_feed(true);
        });
    });
}

#[bench]
fn bench_get_future_feeds(b: &mut Bencher) {
    truncate_db().unwrap();

    b.iter(|| {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let client = Client::configure()
            .connector(HttpsConnector::new(4, &handle).unwrap())
            .build(&handle);

        let mut foo = vec![];

        URLS2.iter().for_each(|url| {
            let s = Source::from_url(url).unwrap();
            let future = s.into_fututre_feed(&client, true);
            foo.push(future);
        });

        let work = join_all(foo);
        let res = core.run(work);
        assert!(res.is_ok());
    })
}
