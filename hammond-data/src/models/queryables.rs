use chrono::prelude::*;
use diesel::prelude::*;
use diesel;

use reqwest;
use diesel::SaveChangesDsl;
use rss::Channel;

use hyper;
use hyper::client::HttpConnector;
use hyper::{Client, Method, Request, Response, StatusCode, Uri};
use hyper::header::{ETag, EntityTag, HttpDate, IfModifiedSince, IfNoneMatch, LastModified};
use hyper_tls::HttpsConnector;

use futures::prelude::*;
// use futures::future::{ok, result};

use schema::{episode, podcast, source};
use feed::Feed;
use errors::*;

use models::insertables::NewSource;
use database::connection;

use std::io::Read;
use std::str::FromStr;

#[derive(Queryable, Identifiable, AsChangeset, Associations, PartialEq)]
#[table_name = "episode"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, podcast_id)]
#[belongs_to(Podcast, foreign_key = "podcast_id")]
#[derive(Debug, Clone)]
/// Diesel Model of the episode table.
pub struct Episode {
    rowid: i32,
    title: String,
    uri: Option<String>,
    local_uri: Option<String>,
    description: Option<String>,
    epoch: i32,
    length: Option<i32>,
    duration: Option<i32>,
    guid: Option<String>,
    played: Option<i32>,
    favorite: bool,
    archive: bool,
    podcast_id: i32,
}

impl Episode {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `title` field.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the `title`.
    pub fn set_title(&mut self, value: &str) {
        self.title = value.to_string();
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(|s| s.as_str())
    }

    /// Set the `uri`.
    pub fn set_uri(&mut self, value: Option<&str>) {
        self.uri = value.map(|x| x.to_string());
    }

    /// Get the value of the `local_uri`.
    ///
    /// Represents the local uri,usually filesystem path,
    /// that the media file will be located at.
    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_ref().map(|s| s.as_str())
    }

    /// Set the `local_uri`.
    pub fn set_local_uri(&mut self, value: Option<&str>) {
        self.local_uri = value.map(|x| x.to_string());
    }

    /// Get the `description`.
    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_str())
    }

    /// Set the `description`.
    pub fn set_description(&mut self, value: Option<&str>) {
        self.description = value.map(|x| x.to_string());
    }

    /// Get the value of the `description`.
    pub fn guid(&self) -> Option<&str> {
        self.guid.as_ref().map(|s| s.as_str())
    }

    /// Set the `guid`.
    pub fn set_guid(&mut self, value: Option<&str>) {
        self.guid = value.map(|x| x.to_string());
    }

    /// Get the `epoch` value.
    ///
    /// Retrieved from the rss Item publish date.
    /// Value is set to Utc whenever possible.
    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    /// Set the `epoch`.
    pub fn set_epoch(&mut self, value: i32) {
        self.epoch = value;
    }

    /// Get the `length`.
    ///
    /// The number represents the size of the file in bytes.
    pub fn length(&self) -> Option<i32> {
        self.length
    }

    /// Set the `length`.
    pub fn set_length(&mut self, value: Option<i32>) {
        self.length = value;
    }

    /// Get the `duration` value.
    ///
    /// The number represents the duration of the item/episode in seconds.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// Set the `duration`.
    pub fn set_duration(&mut self, value: Option<i32>) {
        self.duration = value;
    }

    /// Epoch representation of the last time the episode was played.
    ///
    /// None/Null for unplayed.
    pub fn played(&self) -> Option<i32> {
        self.played
    }

    /// Set the `played` value.
    pub fn set_played(&mut self, value: Option<i32>) {
        self.played = value;
    }

    /// Represents the archiving policy for the episode.
    pub fn archive(&self) -> bool {
        self.archive
    }

    /// Set the `archive` policy.
    ///
    /// If true, the download cleanr will ignore the episode
    /// and the corresponding media value will never be automaticly deleted.
    pub fn set_archive(&mut self, b: bool) {
        self.archive = b
    }

    /// Get the `favorite` status of the `Episode`.
    pub fn favorite(&self) -> bool {
        self.favorite
    }

    /// Set `favorite` status.
    pub fn set_favorite(&mut self, b: bool) {
        self.favorite = b
    }

    /// `Podcast` table foreign key.
    pub fn podcast_id(&self) -> i32 {
        self.podcast_id
    }

    /// Sets the `played` value with the current `epoch` timestap and save it.
    pub fn set_played_now(&mut self) -> Result<()> {
        let epoch = Utc::now().timestamp() as i32;
        self.set_played(Some(epoch));
        self.save()?;
        Ok(())
    }

    /// Helper method to easily save/"sync" current state of self to the Database.
    pub fn save(&self) -> Result<Episode> {
        let db = connection();
        let tempdb = db.get()?;

        Ok(self.save_changes::<Episode>(&*tempdb)?)
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[table_name = "episode"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, podcast_id)]
#[derive(Debug, Clone)]
/// Diesel Model to be used for constructing `EpisodeWidgets`.
pub struct EpisodeWidgetQuery {
    rowid: i32,
    title: String,
    uri: Option<String>,
    local_uri: Option<String>,
    epoch: i32,
    length: Option<i32>,
    duration: Option<i32>,
    played: Option<i32>,
    // favorite: bool,
    // archive: bool,
    podcast_id: i32,
}

impl From<Episode> for EpisodeWidgetQuery {
    fn from(e: Episode) -> EpisodeWidgetQuery {
        EpisodeWidgetQuery {
            rowid: e.rowid,
            title: e.title,
            uri: e.uri,
            local_uri: e.local_uri,
            epoch: e.epoch,
            length: e.length,
            duration: e.duration,
            played: e.played,
            podcast_id: e.podcast_id,
        }
    }
}

impl EpisodeWidgetQuery {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `title` field.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(|s| s.as_str())
    }

    /// Get the value of the `local_uri`.
    ///
    /// Represents the local uri,usually filesystem path,
    /// that the media file will be located at.
    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_ref().map(|s| s.as_str())
    }

    /// Set the `local_uri`.
    pub fn set_local_uri(&mut self, value: Option<&str>) {
        self.local_uri = value.map(|x| x.to_string());
    }

    /// Get the `epoch` value.
    ///
    /// Retrieved from the rss Item publish date.
    /// Value is set to Utc whenever possible.
    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    /// Get the `length`.
    ///
    /// The number represents the size of the file in bytes.
    pub fn length(&self) -> Option<i32> {
        self.length
    }

    /// Set the `length`.
    pub fn set_length(&mut self, value: Option<i32>) {
        self.length = value;
    }

    /// Get the `duration` value.
    ///
    /// The number represents the duration of the item/episode in seconds.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// Set the `duration`.
    pub fn set_duration(&mut self, value: Option<i32>) {
        self.duration = value;
    }

    /// Epoch representation of the last time the episode was played.
    ///
    /// None/Null for unplayed.
    pub fn played(&self) -> Option<i32> {
        self.played
    }

    /// Set the `played` value.
    pub fn set_played(&mut self, value: Option<i32>) {
        self.played = value;
    }

    // /// Represents the archiving policy for the episode.
    // pub fn archive(&self) -> bool {
    //     self.archive
    // }

    // /// Set the `archive` policy.
    // ///
    // /// If true, the download cleanr will ignore the episode
    // /// and the corresponding media value will never be automaticly deleted.
    // pub fn set_archive(&mut self, b: bool) {
    //     self.archive = b
    // }

    // /// Get the `favorite` status of the `Episode`.
    // pub fn favorite(&self) -> bool {
    //     self.favorite
    // }

    // /// Set `favorite` status.
    // pub fn set_favorite(&mut self, b: bool) {
    //     self.favorite = b
    // }

    /// `Podcast` table foreign key.
    pub fn podcast_id(&self) -> i32 {
        self.podcast_id
    }

    /// Sets the `played` value with the current `epoch` timestap and save it.
    pub fn set_played_now(&mut self) -> Result<()> {
        let epoch = Utc::now().timestamp() as i32;
        self.set_played(Some(epoch));
        self.save()?;
        Ok(())
    }

    /// Helper method to easily save/"sync" current state of self to the Database.
    pub fn save(&self) -> Result<usize> {
        use schema::episode::dsl::*;

        let db = connection();
        let tempdb = db.get()?;

        Ok(diesel::update(episode.filter(rowid.eq(self.rowid)))
            .set(self)
            .execute(&*tempdb)?)
    }
}

#[derive(Queryable, AsChangeset, PartialEq)]
#[table_name = "episode"]
#[changeset_options(treat_none_as_null = "true")]
#[primary_key(title, podcast_id)]
#[derive(Debug, Clone)]
/// Diesel Model to be used internal with the `utils::checkup` function.
pub struct EpisodeCleanerQuery {
    rowid: i32,
    local_uri: Option<String>,
    played: Option<i32>,
}

impl From<Episode> for EpisodeCleanerQuery {
    fn from(e: Episode) -> EpisodeCleanerQuery {
        EpisodeCleanerQuery {
            rowid: e.rowid(),
            local_uri: e.local_uri,
            played: e.played,
        }
    }
}

impl EpisodeCleanerQuery {
    /// Get the value of the sqlite's `ROW_ID`
    pub fn rowid(&self) -> i32 {
        self.rowid
    }

    /// Get the value of the `local_uri`.
    ///
    /// Represents the local uri,usually filesystem path,
    /// that the media file will be located at.
    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_ref().map(|s| s.as_str())
    }

    /// Set the `local_uri`.
    pub fn set_local_uri(&mut self, value: Option<&str>) {
        self.local_uri = value.map(|x| x.to_string());
    }

    /// Epoch representation of the last time the episode was played.
    ///
    /// None/Null for unplayed.
    pub fn played(&self) -> Option<i32> {
        self.played
    }

    /// Set the `played` value.
    pub fn set_played(&mut self, value: Option<i32>) {
        self.played = value;
    }

    /// Helper method to easily save/"sync" current state of self to the Database.
    pub fn save(&self) -> Result<usize> {
        use schema::episode::dsl::*;

        let db = connection();
        let tempdb = db.get()?;

        Ok(diesel::update(episode.filter(rowid.eq(self.rowid())))
            .set(self)
            .execute(&*tempdb)?)
    }
}

#[derive(Queryable, Identifiable, AsChangeset, Associations, PartialEq)]
#[belongs_to(Source, foreign_key = "source_id")]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "podcast"]
#[derive(Debug, Clone)]
/// Diesel Model of the podcast table.
pub struct Podcast {
    id: i32,
    title: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    favorite: bool,
    archive: bool,
    always_dl: bool,
    source_id: i32,
}

impl Podcast {
    /// Get the Feed `id`.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the Feed `title`.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the Feed `link`.
    ///
    /// Usually the website/homepage of the content creator.
    pub fn link(&self) -> &str {
        &self.link
    }

    /// Set the Podcast/Feed `link`.
    pub fn set_link(&mut self, value: &str) {
        self.link = value.to_string();
    }

    /// Get the `description`.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Set the `description`.
    pub fn set_description(&mut self, value: &str) {
        self.description = value.to_string();
    }

    /// Get the `image_uri`.
    ///
    /// Represents the uri(url usually) that the Feed cover image is located at.
    pub fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_ref().map(|s| s.as_str())
    }

    /// Set the `image_uri`.
    pub fn set_image_uri(&mut self, value: Option<&str>) {
        self.image_uri = value.map(|x| x.to_string());
    }

    /// Represents the archiving policy for the episode.
    pub fn archive(&self) -> bool {
        self.archive
    }

    /// Set the `archive` policy.
    pub fn set_archive(&mut self, b: bool) {
        self.archive = b
    }

    /// Get the `favorite` status of the `Podcast` Feed.
    pub fn favorite(&self) -> bool {
        self.favorite
    }

    /// Set `favorite` status.
    pub fn set_favorite(&mut self, b: bool) {
        self.favorite = b
    }

    /// Represents the download policy for the `Podcast` Feed.
    ///
    /// Reserved for the use with a Download manager, yet to be implemented.
    ///
    /// If true Podcast Episode should be downloaded automaticly/skipping
    /// the selection queue.
    pub fn always_download(&self) -> bool {
        self.always_dl
    }

    /// Set the download policy.
    pub fn set_always_download(&mut self, b: bool) {
        self.always_dl = b
    }

    /// `Source` table foreign key.
    pub fn source_id(&self) -> i32 {
        self.source_id
    }

    /// Helper method to easily save/"sync" current state of self to the Database.
    pub fn save(&self) -> Result<Podcast> {
        let db = connection();
        let tempdb = db.get()?;

        Ok(self.save_changes::<Podcast>(&*tempdb)?)
    }
}

#[derive(Queryable, Debug, Clone)]
/// Diesel Model of the podcast cover query.
/// Used for fetching information about a Podcast's cover.
pub struct PodcastCoverQuery {
    id: i32,
    title: String,
    image_uri: Option<String>,
}

impl From<Podcast> for PodcastCoverQuery {
    fn from(p: Podcast) -> PodcastCoverQuery {
        PodcastCoverQuery {
            id: p.id(),
            title: p.title,
            image_uri: p.image_uri,
        }
    }
}

impl PodcastCoverQuery {
    /// Get the Feed `id`.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the Feed `title`.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the `image_uri`.
    ///
    /// Represents the uri(url usually) that the Feed cover image is located at.
    pub fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_ref().map(|s| s.as_str())
    }
}

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
