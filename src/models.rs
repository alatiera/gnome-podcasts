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
    thumbnail: Option<String>,
    lenght: Option<i32>,
    guid: Option<String>,
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
    image_local: Option<String>,
}


#[derive(Insertable)]
#[table_name = "episode"]
#[derive(Debug, Clone)]
pub struct NewEpisode<'a> {
    title: &'a str,
    uri: &'a str,
    local_uri: Option<&'a str>,
    description: Option<&'a str>,
    thumbnail: Option<&'a str>,
    lenght: Option<i32>,
    guid: Option<&'a str>,
    podcast_id: i32,
}

#[derive(Insertable)]
#[table_name = "podcast"]
#[derive(Debug, Clone)]
pub struct NewPodcast<'a> {
    title: &'a str,
    uri: &'a str,
    link: Option<&'a str>,
    description: Option<&'a str>,
    last_modified: Option<&'a str>,
    http_etag: Option<&'a str>,
    image_uri: Option<&'a str>,
    image_local: Option<&'a str>,
}