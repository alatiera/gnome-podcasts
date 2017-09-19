use reqwest;
use rss::Channel;

use schema::{episode, podcast, source};
use errors::*;

#[derive(Queryable, Identifiable)]
#[derive(Associations)]
#[table_name = "episode"]
#[belongs_to(Podcast, foreign_key = "podcast_id")]
#[derive(Debug, Clone)]
pub struct Episode {
    id: i32,
    title: String,
    uri: String,
    local_uri: Option<String>,
    description: Option<String>,
    published_date: String,
    epoch: i32,
    length: Option<i32>,
    guid: Option<String>,
    podcast_id: i32,
}

#[derive(Queryable, Identifiable)]
#[derive(Associations)]
#[belongs_to(Source, foreign_key = "source_id")]
#[table_name = "podcast"]
#[derive(Debug, Clone)]
pub struct Podcast {
    id: i32,
    title: String,
    uri: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    source_id: i32,
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
    // This is a mess
    pub fn get_podcast(&mut self) -> Result<NewPodcast> {
        use std::io::Read;
        use reqwest::header::*;
        use std::str::FromStr;

        let mut req = reqwest::get(&self.uri)?;

        let mut buf = String::new();
        req.read_to_string(&mut buf)?;
        // info!("{}", buf);

        let headers = req.headers();
        debug!("{:#?}", headers);

        // for h in headers.iter() {
        //     info!("{}: {}", h.name(), h.value_string());
        // }

        // let etag = headers.get_raw("ETag").unwrap();
        let etag = headers.get::<ETag>();
        let lst_mod = headers.get::<LastModified>();
        info!("Etag: {:?}", etag);
        info!("Last mod: {:?}", lst_mod);

        self.http_etag = etag.map(|x| x.tag().to_string().to_owned());
        self.last_modified = lst_mod.map(|x| format!("{}", x));
        info!("Self etag: {:?}", self.http_etag);
        info!("Self last_mod: {:?}", self.last_modified);

        let chan = Channel::from_str(&buf)?;
        let foo = ::parse_feeds::parse_podcast(&chan, &self.uri)?;

        Ok(foo)
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
    pub title: &'a str,
    pub uri: &'a str,
    pub local_uri: Option<&'a str>,
    pub description: Option<&'a str>,
    pub published_date: &'a str,
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
    pub uri: String,
    pub link: String,
    pub description: String,
    pub image_uri: Option<String>,
}


impl<'a> NewPodcast {
    pub fn from_url(uri: &'a str) -> Result<NewPodcast> {
        let chan = Channel::from_url(uri)?;
        let foo = ::parse_feeds::parse_podcast(&chan, uri)?;
        Ok(foo)
    }
}