extern crate futures;
extern crate hyper;
extern crate tokio_core;

use std::io::{self, Write};
use std::str::FromStr;
use futures::{Future, Stream};
// use futures::future::join_all;
use hyper::Client;
use hyper::client::HttpConnector;
use hyper::Method;
use hyper::Uri;
use tokio_core::reactor::Core;
use hyper_tls::HttpsConnector;
// use errors::*;
// use hyper::header::{ETag, LastModified};

use Source;

#[allow(dead_code)]
fn foo() {
    let uri = "https://www.rust-lang.org/".parse().unwrap();
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);

    let work = client.get(uri).and_then(|res| {
        println!("Response: {}", res.status());

        res.body()
            .for_each(|chunk| io::stdout().write_all(&chunk).map_err(From::from))
    });

    core.run(work).unwrap();
}

#[allow(dead_code)]
fn req_constructor(
    client: &mut Client<HttpsConnector<HttpConnector>>,
    s: &mut Source,
) -> Box<Future<Item = hyper::Response, Error = hyper::Error>> {
    use hyper::header::{EntityTag, HttpDate, IfModifiedSince, IfNoneMatch};

    let uri = Uri::from_str(&s.uri()).unwrap();
    let mut req = hyper::Request::new(Method::Get, uri);

    // if !ignore_etags {
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
    // }

    let work = client.request(req);
    Box::new(work)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::result;
    use rss::Channel;

    use database::truncate_db;
    use Source;

    #[test]
    fn test_foo() {
        foo()
    }

    #[test]
    fn test_bar() {
        truncate_db().unwrap();

        let mut core = Core::new().unwrap();
        let mut client = Client::configure()
            .connector(HttpsConnector::new(4, &core.handle()).unwrap())
            .build(&core.handle());

        let url = "https://feeds.feedburner.com/InterceptedWithJeremyScahill";
        let mut source = Source::from_url(url).unwrap();

        let channel = req_constructor(&mut client, &mut source)
            .map(|res| {
                info!("Status: {}", res.status());
                source.update_etag2(&res);
                res
            })
            .and_then(|res| res.body().concat2())
            .map(|concat2| concat2.into_iter())
            .map(|iter| {
                let utf_8_bytes = iter.collect::<Vec<u8>>();
                let buf = String::from_utf8_lossy(&utf_8_bytes).into_owned();
                Channel::from_str(&buf).unwrap()
            });

        let chan = core.run(channel).unwrap();
        println!("{:?}", chan);
    }
}
