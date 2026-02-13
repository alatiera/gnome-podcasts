use anyhow::Result;
use http_test_server::TestServer;
use http_test_server::http::Status;

pub const MOCK_FEED_DEPROGRAM: &'static str = "/the-deprogram";
pub const MOCK_FEED_DEPROGRAM_OLD: &'static str = "/the-deprogram-old";
pub const MOCK_FEED_INTERCEPTED: &'static str = "/InterceptedWithJeremyScahill";
pub const MOCK_FEED_LINUX_UNPLUGGED: &'static str = "/linuxunplugged";
pub const MOCK_FEED_THE_TIP_OFF: &'static str = "/thetipoff";
pub const MOCK_FEED_THE_STEAL_THE_STARS: &'static str = "/steal-the-stars";
pub const MOCK_FEED_GREATER_THAN_CODE: &'static str = "/greaterthancode";
pub const MOCK_FEED_SERIES_I_CINEMA: &'static str = "/series-i-cinema.xml";

pub fn mock_feed_url(server: &TestServer, feed: &str) -> String {
    format!("http://127.0.0.1:{}{}", server.port(), feed)
}

pub fn mock_feed_server() -> Result<TestServer> {
    let server = TestServer::new()?;
    // "https://rss.art19.com/the-deprogram"
    // redirects -> https://feeds.buzzsprout.com/1890340.rss
    // redirects -> https://rss.buzzsprout.com/1890340.rss
    server
        .create_resource(MOCK_FEED_DEPROGRAM)
        .status(Status::OK)
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("Cache-Control", "no-cache")
        .header("last-modified", "Fri, 13 Feb 2026 11:32:00 GMT")
        .header("etag", "\"f21888f526ba083147754d5a4ac9a0c5\"")
        .header("date", "Fri, 13 Feb 2026 15:03:38 GMT")
        .body(include_str!("../tests/feeds/2026-02-13-deprogram.xml"));

    // https://web.archive.org/web/20220110083840if_/https://rss.art19.com/the-deprogram
    // https://web.archive.org/web/20220120083840if_/https://rss.art19.com/the-deprogram
    server
        .create_resource(MOCK_FEED_DEPROGRAM_OLD)
        .status(Status::OK)
        .header("Content-Type", "text/xml; charset=utf-8")
        .body(include_str!("../tests/feeds/2022-01-10-deprogram.xml"));

    // https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.com/InterceptedWithJeremyScahill
    server
        .create_resource(MOCK_FEED_INTERCEPTED)
        .status(Status::OK)
        .header("Content-Type", "text/xml; charset=utf-8")
        .body(include_str!("../tests/feeds/2018-01-20-Intercepted.xml"));

    // https://web.archive.org/web/20180120110314if_/https://feeds.feedburner.com/linuxunplugged
    server
        .create_resource(MOCK_FEED_LINUX_UNPLUGGED)
        .status(Status::OK)
        .header("Content-Type", "text/xml; charset=utf-8")
        .body(include_str!("../tests/feeds/2018-01-20-LinuxUnplugged.xml"));

    // https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff
    server
        .create_resource(MOCK_FEED_THE_TIP_OFF)
        .status(Status::OK)
        .header("Content-Type", "text/xml; charset=utf-8")
        .body(include_str!("../tests/feeds/2018-01-20-TheTipOff.xml"));

    // https://web.archive.org/web/20180120104957if_/https://rss.art19.com/steal-the-stars
    server
        .create_resource(MOCK_FEED_THE_STEAL_THE_STARS)
        .status(Status::OK)
        .header("Content-Type", "text/xml; charset=utf-8")
        .body(include_str!("../tests/feeds/2018-01-20-StealTheStars.xml"));

    // https://web.archive.org/web/20180120104741if_/https://www.greaterthancode.com/feed/podcast
    server
        .create_resource(MOCK_FEED_GREATER_THAN_CODE)
        .status(Status::OK)
        .header("Content-Type", "text/xml; charset=utf-8")
        .body(include_str!(
            "../tests/feeds/2018-01-20-GreaterThanCode.xml"
        ));

    Ok(server)
}

// TODO The iso-encoding test is unsafe and isolated from the others,
// because the TestServer doesn't support byte responses in body yet,
// only `&'static str` and `String`.
// So I had to load unchecked byte data into a String,
// which usually should only contain valid utf8.
//
// We could open an issue about adding something like .body_bytes here:
// https://github.com/viniciusgerevini/http-test-server/issues
pub unsafe fn iso_encoded_mock_server() -> Result<TestServer> {
    let body = unsafe {
        let raw = include_bytes!("../tests/feeds/2022-series-i-cinema.xml");
        String::from_utf8_unchecked(raw.to_vec())
    };
    let server = TestServer::new()?;

    // https://web.archive.org/web/20220205205130if_/https://dinamics.ccma.cat/public/podcast/catradio/xml/series-i-cinema.xml
    server
        .create_resource(MOCK_FEED_SERIES_I_CINEMA)
        .status(Status::OK)
        .header("Content-Type", "text/xml; charset=iso-8859-1")
        .body_fn(move |_| body.clone());

    Ok(server)
}
