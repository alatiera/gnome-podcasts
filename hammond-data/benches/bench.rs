#![feature(test)]

extern crate hammond_data;
extern crate hyper;
extern crate rand;
// extern crate rayon;
extern crate rss;
extern crate test;

// use rayon::prelude::*;

use test::Bencher;

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
    "https://www.pcper.com/rss/podcasts-mp3.rss",
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

    feeds.iter().for_each(|x| x.index().unwrap());
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
    URLS2.iter().for_each(|url| {
        Source::from_url(url).unwrap();
    });

    b.iter(|| {
        let sources = hammond_data::dbqueries::get_sources().unwrap();
        index_loop(sources);
        println!("I RUN");
    });
}

#[bench]
fn bench_get_future_feeds(b: &mut Bencher) {
    truncate_db().unwrap();
    URLS2.iter().for_each(|url| {
        Source::from_url(url).unwrap();
    });

    b.iter(|| {
        let sources = hammond_data::dbqueries::get_sources().unwrap();
        hammond_data::pipeline::pipeline(sources, false).unwrap();
    })
}
