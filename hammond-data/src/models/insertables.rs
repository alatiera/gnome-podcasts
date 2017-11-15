use schema::{episode, podcast, source};

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
