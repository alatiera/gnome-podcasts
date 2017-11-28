use chrono::prelude::*;

use reqwest;
use diesel::SaveChangesDsl;
use reqwest::header::{ETag, LastModified};
use rss::Channel;

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
#[belongs_to(Podcast, foreign_key = "podcast_id")]
#[derive(Debug, Clone)]
/// Diesel Model of the episode table.
pub struct Episode {
    id: i32,
    title: Option<String>,
    uri: String,
    local_uri: Option<String>,
    description: Option<String>,
    published_date: Option<String>,
    epoch: i32,
    length: Option<i32>,
    guid: Option<String>,
    played: Option<i32>,
    favorite: bool,
    archive: bool,
    podcast_id: i32,
}

impl Episode {
    /// Get the value of the `title` field.
    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|s| s.as_str())
    }

    /// Set the `title`.
    pub fn set_title(&mut self, value: Option<&str>) {
        self.title = value.map(|x| x.to_string());
    }

    /// Get the value of the `uri`.
    ///
    /// Represents the url(usually) that the media file will be located at.
    pub fn uri(&self) -> &str {
        self.uri.as_ref()
    }

    /// Set the `uri`.
    pub fn set_uri(&mut self, value: &str) {
        self.uri = value.to_string();
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

    /// Get the the `published_date`.
    pub fn published_date(&self) -> Option<&str> {
        self.published_date.as_ref().map(|s| s.as_str())
    }

    /// Set the `published_date`.
    pub fn set_published_date(&mut self, value: Option<&str>) {
        self.published_date = value.map(|x| x.to_string().to_owned());
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
    pub fn length(&self) -> Option<i32> {
        self.length
    }

    /// Set the `length`.
    pub fn set_length(&mut self, value: Option<i32>) {
        self.length = value;
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

impl<'a> Source {
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

    /// Extract Etag and LastModifier from req, and update self and the
    /// corresponding db row.
    fn update_etag(&mut self, req: &reqwest::Response) -> Result<()> {
        let headers = req.headers();

        // let etag = headers.get_raw("ETag").unwrap();
        let etag = headers.get::<ETag>();
        let lmod = headers.get::<LastModified>();

        // FIXME: This dsnt work most of the time apparently
        if self.http_etag() != etag.map(|x| x.tag())
            || self.last_modified != lmod.map(|x| format!("{}", x))
        {
            self.http_etag = etag.map(|x| x.tag().to_string().to_owned());
            self.last_modified = lmod.map(|x| format!("{}", x));
            self.save()?;
        }

        Ok(())
    }

    /// Helper method to easily save/"sync" current state of self to the Database.
    pub fn save(&self) -> Result<Source> {
        let db = connection();
        let tempdb = db.get()?;

        Ok(self.save_changes::<Source>(&*tempdb)?)
    }

    /// `Feed` constructor.
    ///
    /// Fetches the latest xml Feed.
    ///
    /// Updates the validator Http Headers.
    ///
    /// Consumes `self` and Returns the corresponding `Feed` Object.
    // TODO: Refactor into TryInto once it lands on stable.
    pub fn into_feed(mut self) -> Result<Feed> {
        use reqwest::header::{ETag, EntityTag, Headers, HttpDate, LastModified};

        let mut headers = Headers::new();

        if let Some(foo) = self.http_etag() {
            headers.set(ETag(EntityTag::new(true, foo.to_owned())));
        }

        if let Some(foo) = self.last_modified() {
            if let Ok(x) = foo.parse::<HttpDate>() {
                headers.set(LastModified(x));
            }
        }

        // FIXME: I have fucked up somewhere here.
        // Getting back 200 codes even though I supposedly sent etags.
        // info!("Headers: {:?}", headers);
        let client = reqwest::Client::builder().referer(false).build()?;
        let mut req = client.get(self.uri()).headers(headers).send()?;

        info!("GET to {} , returned: {}", self.uri(), req.status());

        // TODO match on more stuff
        // 301: Permanent redirect of the url
        // 302: Temporary redirect of the url
        // 304: Up to date Feed, checked with the Etag
        // 410: Feed deleted
        // match req.status() {
        //     reqwest::StatusCode::NotModified => (),
        //     _ => (),
        // };

        self.update_etag(&req)?;

        let mut buf = String::new();
        req.read_to_string(&mut buf)?;
        let chan = Channel::from_str(&buf)?;

        Ok(Feed::from_channel_source(chan, self))
    }

    /// Construct a new `Source` with the given `uri` and index it.
    pub fn from_url(uri: &str) -> Result<Source> {
        NewSource::new_with_uri(uri).into_source()
    }
}
