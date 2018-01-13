use rss;

use hyper;
use hyper::Client;
use hyper::client::HttpConnector;
use hyper::Method;
use hyper::Uri;
use hyper_tls::HttpsConnector;
// use hyper::header::{ETag, LastModified};
// use hyper::header::{ETag, LastModified};

use futures::{Future, Stream};
// use futures::future::join_all;

// use std::io::{self, Write};
use std::str::FromStr;

use Source;
// use errors::*;

#[allow(dead_code)]
fn request_constructor(
    s: &Source,
    client: &mut Client<HttpsConnector<HttpConnector>>,
    ignore_etags: bool,
) -> Box<Future<Item = hyper::Response, Error = hyper::Error>> {
    use hyper::header::{EntityTag, HttpDate, IfModifiedSince, IfNoneMatch};

    let uri = Uri::from_str(&s.uri()).unwrap();
    let mut req = hyper::Request::new(Method::Get, uri);

    if !ignore_etags {
        if let Some(foo) = s.http_etag() {
            req.headers_mut().set(IfNoneMatch::Items(vec![
                EntityTag::new(true, foo.to_owned()),
            ]));
        }

        if let Some(foo) = s.last_modified() {
            if let Ok(x) = foo.parse::<HttpDate>() {
                req.headers_mut().set(IfModifiedSince(x));
            }
        }
    }

    let work = client.request(req);
    Box::new(work)
}

#[allow(dead_code)]
fn res_to_channel(res: hyper::Response) -> Box<Future<Item = rss::Channel, Error = hyper::Error>> {
    let chan = res.body().concat2().map(|x| x.into_iter()).map(|iter| {
        let utf_8_bytes = iter.collect::<Vec<u8>>();
        let buf = String::from_utf8_lossy(&utf_8_bytes).into_owned();
        rss::Channel::from_str(&buf).unwrap()
    });
    // .map_err(|_| ());
    Box::new(chan)
}

#[cfg(test)]
mod tests {
    use super::*;
    // use futures::future::result;
    use tokio_core::reactor::Core;

    use database::truncate_db;
    use Source;
    // use feed::Feed;

    #[test]
    fn test_bar() {
        truncate_db().unwrap();

        let mut core = Core::new().unwrap();
        let mut client = Client::configure()
            .connector(HttpsConnector::new(4, &core.handle()).unwrap())
            .build(&core.handle());

        let url = "https://feeds.feedburner.com/InterceptedWithJeremyScahill";
        let mut source = Source::from_url(url).unwrap();

        let channel = request_constructor(&source, &mut client, false)
            .map(|res| {
                println!("Status: {}", res.status());
                source.update_etag2(&res).unwrap();
                res
            })
            .and_then(|res| res_to_channel(res));
        // .map(|chan| Feed::from_channel_source(chan, source));

        let chan = core.run(channel).unwrap();

        // let c = chan.wait().unwrap();
        println!("{:?}", chan);
    }
}
