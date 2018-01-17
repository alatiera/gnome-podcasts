use diesel::SaveChangesDsl;
use reqwest;
use rss::Channel;

use hyper;
use hyper::{Client, Method, Request, Response, StatusCode, Uri};
use hyper::client::HttpConnector;
use hyper::header::{ETag, EntityTag, HttpDate, IfModifiedSince, IfNoneMatch, LastModified};
use hyper_tls::HttpsConnector;

use futures::prelude::*;
// use futures::future::{ok, result};

use database::connection;
use errors::*;
use feed::Feed;
use models::NewSource;
use schema::source;

use std::io::Read;
use std::str::FromStr;

#[derive(Queryable, Identifiable, AsChangeset, PartialEq)]
#[table_name = "source"]
#[changeset_options(treat_none_as_null = "true")]
#[derive(Debug, Clone)]
/// Diesel Model of the source table.
pub struct Source {
    id: i32,
    uri: String,
    /// FIXME
    pub last_modified: Option<String>,
    /// FIXME
    pub http_etag: Option<String>,
}

impl Source {
    /// Get the source `id` column.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Represents the location(usually url) of the Feed xml file.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Represents the Http Last-Modified Header field.
    ///
    /// See [RFC 7231](https://tools.ietf.org/html/rfc7231#section-7.2) for more.
    pub fn last_modified(&self) -> Option<&str> {
        self.last_modified.as_ref().map(|s| s.as_str())
    }

    /// Set `last_modified` value.
    pub fn set_last_modified(&mut self, value: Option<&str>) {
        self.last_modified = value.map(|x| x.to_string());
    }

    /// Represents the Http Etag Header field.
    ///
    /// See [RFC 7231](https://tools.ietf.org/html/rfc7231#section-7.2) for more.
    pub fn http_etag(&self) -> Option<&str> {
        self.http_etag.as_ref().map(|s| s.as_str())
    }

    /// Set `http_etag` value.
    pub fn set_http_etag(&mut self, value: Option<&str>) {
        self.http_etag = value.map(|x| x.to_string());
    }

    /// Helper method to easily save/"sync" current state of self to the Database.
    pub fn save(&self) -> Result<Source> {
        let db = connection();
        let tempdb = db.get()?;

        Ok(self.save_changes::<Source>(&*tempdb)?)
    }

    /// Extract Etag and LastModifier from res, and update self and the
    /// corresponding db row.
    fn update_etag(&mut self, res: &reqwest::Response) -> Result<()> {
        let headers = res.headers();

        let etag = headers.get::<ETag>();
        let lmod = headers.get::<LastModified>();

        if self.http_etag() != etag.map(|x| x.tag()) || self.last_modified != lmod.map(|x| {
            format!("{}", x)
        }) {
            self.http_etag = etag.map(|x| x.tag().to_string().to_owned());
            self.last_modified = lmod.map(|x| format!("{}", x));
            self.save()?;
        }

        Ok(())
    }

    /// Extract Etag and LastModifier from res, and update self and the
    /// corresponding db row.
    fn update_etag2(mut self, res: &Response) -> Result<()> {
        let headers = res.headers();

        let etag = headers.get::<ETag>();
        let lmod = headers.get::<LastModified>();

        if self.http_etag() != etag.map(|x| x.tag()) || self.last_modified != lmod.map(|x| {
            format!("{}", x)
        }) {
            self.http_etag = etag.map(|x| x.tag().to_string().to_owned());
            self.last_modified = lmod.map(|x| format!("{}", x));
            self.save()?;
        }

        Ok(())
    }

    /// `Feed` constructor.
    ///
    /// Fetches the latest xml Feed.
    ///
    /// Updates the validator Http Headers.
    ///
    /// Consumes `self` and Returns the corresponding `Feed` Object.
    // TODO: Refactor into TryInto once it lands on stable.
    pub fn into_feed(&mut self, ignore_etags: bool) -> Result<Feed> {
        use reqwest::header::{EntityTag, Headers, HttpDate, IfModifiedSince, IfNoneMatch};

        let mut headers = Headers::new();

        if !ignore_etags {
            if let Some(foo) = self.http_etag() {
                headers.set(IfNoneMatch::Items(vec![
                    EntityTag::new(true, foo.to_owned()),
                ]));
            }

            if let Some(foo) = self.last_modified() {
                if let Ok(x) = foo.parse::<HttpDate>() {
                    headers.set(IfModifiedSince(x));
                }
            }
        }

        let client = reqwest::Client::builder().referer(false).build()?;
        let mut res = client.get(self.uri()).headers(headers).send()?;

        info!("GET to {} , returned: {}", self.uri(), res.status());

        self.update_etag(&res)?;
        match_status(res.status())?;

        let mut buf = String::new();
        res.read_to_string(&mut buf)?;
        let chan = Channel::from_str(&buf)?;

        Ok(Feed::from_channel_source(chan, self.id))
    }

    // FIXME:
    /// Docs
    pub fn into_fututre_feed(
        self,
        client: &Client<HttpsConnector<HttpConnector>>,
        ignore_etags: bool,
    ) -> Box<Future<Item = Feed, Error = Error>> {
        let id = self.id();
        let feed = request_constructor(&self, client, ignore_etags)
            .map_err(From::from)
            .and_then(move |res| {
                self.update_etag2(&res)?;
                Ok(res)
            })
            .and_then(|res| -> Result<Response> {
                match_status(res.status())?;
                Ok(res)
            })
            .and_then(|res| response_to_channel(res))
            .map(move |chan| Feed::from_channel_source(chan, id));

        Box::new(feed)
    }

    /// Construct a new `Source` with the given `uri` and index it.
    ///
    /// This only indexes the `Source` struct, not the Podcast Feed.
    pub fn from_url(uri: &str) -> Result<Source> {
        NewSource::new(uri).into_source()
    }
}

// TODO: make ignore_etags an Enum for better ergonomics.
// #bools_are_just_2variant_enmus
fn request_constructor(
    s: &Source,
    client: &Client<HttpsConnector<HttpConnector>>,
    ignore_etags: bool,
) -> Box<Future<Item = Response, Error = hyper::Error>> {
    // FIXME: remove unwrap somehow
    let uri = Uri::from_str(&s.uri()).unwrap();
    let mut req = Request::new(Method::Get, uri);

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

    let work = client.request(req).map_err(From::from);
    Box::new(work)
}

fn response_to_channel(res: Response) -> Box<Future<Item = Channel, Error = Error>> {
    let chan = res.body()
        .concat2()
        .map(|x| x.into_iter())
        .map_err(From::from)
        .and_then(|iter| {
            let utf_8_bytes = iter.collect::<Vec<u8>>();
            let buf = String::from_utf8_lossy(&utf_8_bytes).into_owned();
            let chan = Channel::from_str(&buf).map_err(From::from);
            chan
        });
    Box::new(chan)
}

// TODO match on more stuff
// 301: Moved Permanently
// 304: Up to date Feed, checked with the Etag
// 307: Temporary redirect of the url
// 308: Permanent redirect of the url
// 401: Unathorized
// 403: Forbidden
// 408: Timeout
// 410: Feed deleted
fn match_status(code: StatusCode) -> Result<()> {
    match code {
        StatusCode::NotModified => bail!("304: skipping.."),
        StatusCode::TemporaryRedirect => debug!("307: Temporary Redirect."),
        // TODO: Change the source uri to the new one
        StatusCode::MovedPermanently | StatusCode::PermanentRedirect => {
            warn!("Feed was moved permanently.")
        }
        StatusCode::Unauthorized => bail!("401: Unauthorized."),
        StatusCode::Forbidden => bail!("403: Forbidden."),
        StatusCode::NotFound => bail!("404: Not found."),
        StatusCode::RequestTimeout => bail!("408: Request Timeout."),
        StatusCode::Gone => bail!("410: Feed was deleted."),
        _ => (),
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_core::reactor::Core;

    use database::truncate_db;

    #[test]
    fn test_into_future_feed() {
        truncate_db().unwrap();

        let mut core = Core::new().unwrap();
        let client = Client::configure()
            .connector(HttpsConnector::new(4, &core.handle()).unwrap())
            .build(&core.handle());

        let url = "http://www.newrustacean.com/feed.xml";
        let source = Source::from_url(url).unwrap();

        let feed = source.into_fututre_feed(&client, true);

        assert!(core.run(feed).is_ok());
    }
}
