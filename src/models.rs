use reqwest;
use rss::Channel;
use diesel::SaveChangesDsl;
use SqliteConnection;
use reqwest::header::{ETag, LastModified};

use schema::{episode, podcast, source};
use errors::*;

#[derive(Queryable, Identifiable, AsChangeset)]
#[derive(Associations)]
#[table_name = "episode"]
#[belongs_to(Podcast, foreign_key = "podcast_id")]
#[derive(Debug, Clone)]
pub struct Episode {
    id: i32,
    title: Option<String>,
    uri: Option<String>,
    local_uri: Option<String>,
    description: Option<String>,
    published_date: Option<String>,
    epoch: Option<i32>,
    length: Option<i32>,
    guid: Option<String>,
    podcast_id: i32,
}

impl Episode {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|s| s.as_str())
    }

    pub fn set_title(&mut self, value: Option<&str>) {
        self.title = value.map(|x| x.to_string());
    }

    pub fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(|s| s.as_str())
    }

    pub fn set_uri(&mut self, value: Option<&str>) {
        self.uri = value.map(|x| x.to_string());
    }

    pub fn local_uri(&self) -> Option<&str> {
        self.local_uri.as_ref().map(|s| s.as_str())
    }

    pub fn set_local_uri(&mut self, value: Option<&str>) {
        self.local_uri = value.map(|x| x.to_string());
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_str())
    }

    pub fn set_description(&mut self, value: Option<&str>) {
        self.description = value.map(|x| x.to_string());
    }

    pub fn published_date(&self) -> Option<&str> {
        self.published_date.as_ref().map(|s| s.as_str())
    }

    pub fn set_published_date(&mut self, value: Option<&str>) {
        self.published_date = value.map(|x| x.to_string());
    }

    pub fn guid(&self) -> Option<&str> {
        self.guid.as_ref().map(|s| s.as_str())
    }

    pub fn set_guid(&mut self, value: Option<&str>) {
        self.guid = value.map(|x| x.to_string());
    }

    pub fn epoch(&self) -> Option<i32> {
        self.epoch
    }

    pub fn set_epoch(&mut self, value: Option<i32>) {
        self.epoch = value;
    }

    pub fn length(&self) -> Option<i32> {
        self.length
    }

    pub fn set_length(&mut self, value: Option<i32>) {
        self.length = value;
    }
}

#[derive(Queryable, Identifiable, AsChangeset)]
#[derive(Associations)]
#[belongs_to(Source, foreign_key = "source_id")]
#[table_name = "podcast"]
#[derive(Debug, Clone)]
pub struct Podcast {
    id: i32,
    title: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    source_id: i32,
}

impl Podcast {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn link(&self) -> &str {
        &self.link
    }

    pub fn set_link(&mut self, value: &str) {
        self.link = value.to_string();
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn set_description(&mut self, value: &str) {
        self.description = value.to_string();
    }

    pub fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_ref().map(|s| s.as_str())
    }

    pub fn set_image_uri(&mut self, value: Option<&str>) {
        self.image_uri = value.map(|x| x.to_string());
    }
}

#[derive(Queryable, Identifiable, AsChangeset)]
#[table_name = "source"]
#[derive(Debug, Clone)]
pub struct Source {
    id: i32,
    uri: String,
    last_modified: Option<String>,
    http_etag: Option<String>,
}

impl<'a> Source {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn last_modified(&self) -> Option<&str> {
        self.last_modified.as_ref().map(|s| s.as_str())
    }

    pub fn set_last_modified(&mut self, value: Option<&str>) {
        self.last_modified = value.map(|x| x.to_string());
    }

    pub fn http_etag(&self) -> Option<&str> {
        self.http_etag.as_ref().map(|s| s.as_str())
    }

    pub fn set_http_etag(&mut self, value: Option<&str>) {
        self.http_etag = value.map(|x| x.to_string());
    }

    /// Fetch the xml feed from the source url, update the etag headers,
    /// parse the feed into an rss:Channel and return it.
    pub fn get_podcast_chan(&mut self, con: &SqliteConnection) -> Result<Channel> {
        use std::io::Read;
        use std::str::FromStr;

        let mut req = reqwest::get(&self.uri)?;
        self.update_etag(con, &req)?;

        let mut buf = String::new();
        // info!("{}", buf);
        req.read_to_string(&mut buf)?;
        let chan = Channel::from_str(&buf)?;

        Ok(chan)
    }

    /// Extract Etag and LastModifier from req, and update self and the corresponding db row.
    fn update_etag(&mut self, con: &SqliteConnection, req: &reqwest::Response) -> Result<()> {

        let headers = req.headers();
        debug!("{:#?}", headers);

        // let etag = headers.get_raw("ETag").unwrap();
        let etag = headers.get::<ETag>();
        let lmod = headers.get::<LastModified>();

        self.http_etag = etag.map(|x| x.tag().to_string().to_owned());
        self.last_modified = lmod.map(|x| format!("{}", x));
        info!("Self etag: {:?}", self.http_etag);
        info!("Self last_mod: {:?}", self.last_modified);

        self.save_changes::<Source>(con)?;
        Ok(())
    }
}

// TODO: Remove pub fields and add setters.
#[derive(Insertable)]
#[table_name = "source"]
#[derive(Debug, Clone)]
pub struct NewSource<'a> {
    pub uri: &'a str,
    pub last_modified: Option<&'a str>,
    pub http_etag: Option<&'a str>,
}

impl<'a> NewSource<'a> {
    pub fn new_with_uri(uri: &'a str) -> NewSource {
        NewSource {
            uri,
            last_modified: None,
            http_etag: None,
        }
    }
}

#[derive(Insertable)]
#[table_name = "episode"]
#[derive(Debug, Clone)]
pub struct NewEpisode<'a> {
    pub title: Option<&'a str>,
    pub uri: Option<&'a str>,
    pub local_uri: Option<&'a str>,
    pub description: Option<&'a str>,
    pub published_date: Option<&'a str>,
    pub length: Option<i32>,
    pub guid: Option<&'a str>,
    pub epoch: i32,
    pub podcast_id: i32,
}

#[derive(Insertable)]
#[table_name = "podcast"]
#[derive(Debug, Clone)]
pub struct NewPodcast {
    pub title: String,
    pub link: String,
    pub description: String,
    pub image_uri: Option<String>,
    pub source_id: i32,
}

impl NewPodcast {
    pub fn from_url(uri: &str, parent: &Source) -> Result<NewPodcast> {
        let chan = Channel::from_url(uri)?;
        let foo = ::feedparser::parse_podcast(&chan, parent.id())?;
        Ok(foo)
    }
}