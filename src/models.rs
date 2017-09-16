use schema::{episode, podcast};

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
    thumbnail: Option<String>,
    length: Option<i32>,
    guid: Option<String>,
    epoch: i32,
    podcast_id: i32,
}

#[derive(Queryable, Identifiable)]
#[table_name = "podcast"]
#[derive(Debug, Clone)]
pub struct Podcast {
    id: i32,
    title: String,
    uri: String,
    link: Option<String>,
    description: Option<String>,
    last_modified: Option<String>,
    http_etag: Option<String>,
    image_uri: Option<String>,
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
pub struct NewPodcast<'a> {
    pub title: &'a str,
    pub uri: &'a str,
    pub link: Option<&'a str>,
    pub description: Option<&'a str>,
    pub last_modified: Option<&'a str>,
    pub http_etag: Option<&'a str>,
    pub image_uri: Option<&'a str>,
}