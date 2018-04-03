use diesel::SaveChangesDsl;
// use failure::ResultExt;
use rss::Channel;
use url::Url;

use hyper::{Client, Method, Request, Response, StatusCode, Uri};
use hyper::client::HttpConnector;
use hyper::header::{ETag, EntityTag, HttpDate, IfModifiedSince, IfNoneMatch, LastModified,
                    Location, UserAgent};
use hyper_tls::HttpsConnector;

// use futures::future::ok;
use futures::prelude::*;
use futures_cpupool::CpuPool;

use database::connection;
use errors::DataError;
use feed::{Feed, FeedBuilder};
use models::{NewSource, Save};
use schema::source;

use std::str::FromStr;

#[derive(Queryable, Identifiable, AsChangeset, PartialEq)]
#[table_name = "source"]
#[changeset_options(treat_none_as_null = "true")]
#[derive(Debug, Clone)]
/// Diesel Model of the source table.
pub struct Source {
    id: i32,
    uri: String,
    last_modified: Option<String>,
    http_etag: Option<String>,
}

impl Save<Source, DataError> for Source {
    /// Helper method to easily save/"sync" current state of self to the
    /// Database.
    fn save(&self) -> Result<Source, DataError> {
        let db = connection();
        let con = db.get()?;

        self.save_changes::<Source>(&con).map_err(From::from)
    }
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

    /// Set the `uri` field value.
    pub fn set_uri(&mut self, uri: String) {
        self.uri = uri;
    }

    /// Represents the Http Last-Modified Header field.
    ///
    /// See [RFC 7231](https://tools.ietf.org/html/rfc7231#section-7.2) for more.
    pub fn last_modified(&self) -> Option<&str> {
        self.last_modified.as_ref().map(|s| s.as_str())
    }

    /// Set `last_modified` value.
    pub fn set_last_modified(&mut self, value: Option<String>) {
        // self.last_modified = value.map(|x| x.to_string());
        self.last_modified = value;
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

    /// Extract Etag and LastModifier from res, and update self and the
    /// corresponding db row.
    fn update_etag(&mut self, res: &Response) -> Result<(), DataError> {
        let headers = res.headers();

        let etag = headers.get::<ETag>().map(|x| x.tag());
        let lmod = headers.get::<LastModified>().map(|x| format!("{}", x));

        if (self.http_etag() != etag) || (self.last_modified != lmod) {
            self.set_http_etag(etag);
            self.set_last_modified(lmod);
            self.save()?;
        }

        Ok(())
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
    // TODO: Rething this api,
    fn match_status(mut self, res: Response) -> Result<(Self, Response), DataError> {
        self.update_etag(&res)?;
        let code = res.status();
        match code {
            StatusCode::NotModified => {
                let err = DataError::HttpStatusError {
                    url: self.uri,
                    status_code: code,
                    context: "304: skipping..".into(),
                };

                return Err(err);
            }
            StatusCode::MovedPermanently => {
                error!("Feed was moved permanently.");
                self.handle_301(&res)?;

                let err = DataError::HttpStatusError {
                    url: self.uri,
                    status_code: code,
                    context: "301: Feed was moved permanently.".into(),
                };

                return Err(err);
            }
            StatusCode::TemporaryRedirect => debug!("307: Temporary Redirect."),
            StatusCode::PermanentRedirect => warn!("308: Permanent Redirect."),
            StatusCode::Unauthorized => {
                let err = DataError::HttpStatusError {
                    url: self.uri,
                    status_code: code,
                    context: "401: Unauthorized.".into(),
                };

                return Err(err);
            }
            StatusCode::Forbidden => {
                let err = DataError::HttpStatusError {
                    url: self.uri,
                    status_code: code,
                    context: "403:  Forbidden.".into(),
                };

                return Err(err);
            }
            StatusCode::NotFound => {
                let err = DataError::HttpStatusError {
                    url: self.uri,
                    status_code: code,
                    context: "404: Not found.".into(),
                };

                return Err(err);
            }
            StatusCode::RequestTimeout => {
                let err = DataError::HttpStatusError {
                    url: self.uri,
                    status_code: code,
                    context: "408: Request Timeout.".into(),
                };

                return Err(err);
            }
            StatusCode::Gone => {
                let err = DataError::HttpStatusError {
                    url: self.uri,
                    status_code: code,
                    context: "410: Feed was deleted..".into(),
                };

                return Err(err);
            }
            _ => info!("HTTP StatusCode: {}", code),
        };
        Ok((self, res))
    }

    fn handle_301(&mut self, res: &Response) -> Result<(), DataError> {
        let headers = res.headers();

        if let Some(url) = headers.get::<Location>() {
            self.set_uri(url.to_string());
            self.http_etag = None;
            self.last_modified = None;
            self.save()?;
            info!("Feed url was updated succesfully.");
            // TODO: Refresh in place instead of next time, Not a priority.
            info!("New content will be fetched with the next refesh.");
        }

        Ok(())
    }

    /// Construct a new `Source` with the given `uri` and index it.
    ///
    /// This only indexes the `Source` struct, not the Podcast Feed.
    pub fn from_url(uri: &str) -> Result<Source, DataError> {
        let url = Url::parse(uri)?;

        NewSource::new(&url).to_source()
    }

    /// `Feed` constructor.
    ///
    /// Fetches the latest xml Feed.
    ///
    /// Updates the validator Http Headers.
    ///
    /// Consumes `self` and Returns the corresponding `Feed` Object.
    // Refactor into TryInto once it lands on stable.
    pub fn into_feed(
        self,
        client: &Client<HttpsConnector<HttpConnector>>,
        pool: CpuPool,
        ignore_etags: bool,
    ) -> Box<Future<Item = Feed, Error = DataError>> {
        let id = self.id();
        let feed = self.request_constructor(client, ignore_etags)
            .and_then(move |(_, res)| response_to_channel(res, pool))
            .and_then(move |chan| {
                FeedBuilder::default()
                    .channel(chan)
                    .source_id(id)
                    .build()
                    .map_err(From::from)
            });

        Box::new(feed)
    }

    // TODO: make ignore_etags an Enum for better ergonomics.
    // #bools_are_just_2variant_enmus
    fn request_constructor(
        self,
        client: &Client<HttpsConnector<HttpConnector>>,
        ignore_etags: bool,
    ) -> Box<Future<Item = (Self, Response), Error = DataError>> {
        // FIXME: remove unwrap somehow
        let uri = Uri::from_str(self.uri()).unwrap();
        let mut req = Request::new(Method::Get, uri);

        // Set the user agent as a fix for issue #53
        // TODO: keep this in sync with tor-browser releases
        req.headers_mut().set(UserAgent::new(
            "Mozilla/5.0 (Windows NT 6.1; rv:52.0) Gecko/20100101 Firefox/52.0",
        ));

        if !ignore_etags {
            if let Some(foo) = self.http_etag() {
                req.headers_mut().set(IfNoneMatch::Items(vec![
                    EntityTag::new(true, foo.to_owned()),
                ]));
            }

            if let Some(foo) = self.last_modified() {
                if let Ok(x) = foo.parse::<HttpDate>() {
                    req.headers_mut().set(IfModifiedSince(x));
                }
            }
        }

        let work = client
            .request(req)
            .map_err(From::from)
            // TODO: tail recursion loop that would follow redirects directly
            .and_then(move |res| self.match_status(res));
        Box::new(work)
    }
}

#[allow(needless_pass_by_value)]
fn response_to_channel(
    res: Response,
    pool: CpuPool,
) -> Box<Future<Item = Channel, Error = DataError> + Send> {
    let chan = res.body()
        .concat2()
        .map(|x| x.into_iter())
        .map_err(From::from)
        .map(|iter| iter.collect::<Vec<u8>>())
        .map(|utf_8_bytes| String::from_utf8_lossy(&utf_8_bytes).into_owned())
        .and_then(|buf| Channel::from_str(&buf).map_err(From::from));

    let cpu_chan = pool.spawn(chan);
    Box::new(cpu_chan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_core::reactor::Core;

    use database::truncate_db;
    use utils::get_feed;

    #[test]
    fn test_into_feed() {
        truncate_db().unwrap();

        let pool = CpuPool::new_num_cpus();
        let mut core = Core::new().unwrap();
        let client = Client::configure()
            .connector(HttpsConnector::new(4, &core.handle()).unwrap())
            .build(&core.handle());

        let url = "https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.\
                   com/InterceptedWithJeremyScahill";
        let source = Source::from_url(url).unwrap();
        let id = source.id();

        let feed = source.into_feed(&client, pool.clone(), true);
        let feed = core.run(feed).unwrap();

        let expected = get_feed("tests/feeds/2018-01-20-Intercepted.xml", id);
        assert_eq!(expected, feed);
    }
}
