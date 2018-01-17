use diesel::SaveChangesDsl;

// use futures::future::{ok, result};

use schema::podcast;
use errors::*;
use database::connection;
use models::Source;

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
