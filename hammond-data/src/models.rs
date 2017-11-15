use reqwest;
use diesel::SaveChangesDsl;
use diesel::result::QueryResult;
use reqwest::header::{ETag, LastModified};

use schema::{episode, podcast, source};
use index_feed::Database;
use errors::*;

#[derive(Queryable, Identifiable, AsChangeset, Associations)]
#[table_name = "episode"]
#[changeset_options(treat_none_as_null = "true")]
#[belongs_to(Podcast, foreign_key = "podcast_id")]
#[derive(Debug, Clone)]
pub struct Episode {
    id: i32,
    title: Option<String>,
    uri: String,
    local_uri: Option<String>,
    description: Option<String>,
    published_date: Option<String>,
    /// Representation of system time. Should be in UTC.
    epoch: i32,
    length: Option<i32>,
    guid: Option<String>,
    /// Represent the epoch value of when the episode was last played.
    played: Option<i32>,
    favorite: bool,
    archive: bool,
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

    /// uri is guaranted to exist based on the db rules
    pub fn uri(&self) -> &str {
        self.uri.as_ref()
    }

    pub fn set_uri(&mut self, value: &str) {
        self.uri = value.to_string();
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
        self.published_date = value.map(|x| x.to_string().to_owned());
    }

    pub fn guid(&self) -> Option<&str> {
        self.guid.as_ref().map(|s| s.as_str())
    }

    pub fn set_guid(&mut self, value: Option<&str>) {
        self.guid = value.map(|x| x.to_string());
    }

    pub fn epoch(&self) -> i32 {
        self.epoch
    }

    pub fn set_epoch(&mut self, value: i32) {
        self.epoch = value;
    }

    pub fn length(&self) -> Option<i32> {
        self.length
    }

    pub fn set_length(&mut self, value: Option<i32>) {
        self.length = value;
    }

    pub fn played(&self) -> Option<i32> {
        self.played
    }

    pub fn set_played(&mut self, value: Option<i32>) {
        self.played = value;
    }

    pub fn archive(&self) -> bool {
        self.archive
    }

    pub fn set_archive(&mut self, b: bool) {
        self.archive = b
    }

    pub fn favorite(&self) -> bool {
        self.favorite
    }

    pub fn set_favorite(&mut self, b: bool) {
        self.favorite = b
    }

    pub fn save(&self, db: &Database) -> QueryResult<Episode> {
        let tempdb = db.lock().unwrap();
        self.save_changes::<Episode>(&*tempdb)
    }
}

#[derive(Queryable, Identifiable, AsChangeset, Associations)]
#[belongs_to(Source, foreign_key = "source_id")]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "podcast"]
#[derive(Debug, Clone)]
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
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn source_id(&self) -> i32 {
        self.source_id
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

    pub fn archive(&self) -> bool {
        self.archive
    }

    pub fn set_archive(&mut self, b: bool) {
        self.archive = b
    }

    pub fn favorite(&self) -> bool {
        self.favorite
    }

    pub fn set_favorite(&mut self, b: bool) {
        self.favorite = b
    }

    pub fn always_download(&self) -> bool {
        self.always_dl
    }

    pub fn set_always_download(&mut self, b: bool) {
        self.always_dl = b
    }

    pub fn save(&self, db: &Database) -> QueryResult<Podcast> {
        let tempdb = db.lock().unwrap();
        self.save_changes::<Podcast>(&*tempdb)
    }
}

#[derive(Queryable, Identifiable, AsChangeset)]
#[table_name = "source"]
#[changeset_options(treat_none_as_null = "true")]
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

    /// Extract Etag and LastModifier from req, and update self and the
    /// corresponding db row.
    pub fn update_etag(&mut self, db: &Database, req: &reqwest::Response) -> Result<()> {
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
            self.save(db)?;
        }

        Ok(())
    }

    pub fn save(&self, db: &Database) -> QueryResult<Source> {
        let tempdb = db.lock().unwrap();
        self.save_changes::<Source>(&*tempdb)
    }
}

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
    pub description: Option<&'a str>,
    pub published_date: Option<String>,
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
    // This is meant only to be used to make unit tests easier.
    pub fn into_podcast(self) -> Podcast {
        Podcast {
            id: 0,
            title: self.title,
            link: self.link,
            description: self.description,
            image_uri: self.image_uri,
            source_id: self.source_id,
            always_dl: false,
            archive: false,
            favorite: false,
        }
    }
}
