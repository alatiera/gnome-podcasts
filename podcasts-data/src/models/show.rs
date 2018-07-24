use diesel::SaveChangesDsl;

use database::connection;
use errors::DataError;
use models::{Save, Source};
use schema::shows;

#[derive(Queryable, Identifiable, AsChangeset, Associations, PartialEq)]
#[belongs_to(Source, foreign_key = "source_id")]
#[changeset_options(treat_none_as_null = "true")]
#[table_name = "shows"]
#[derive(Debug, Clone)]
/// Diesel Model of the shows table.
pub struct Show {
    id: i32,
    title: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    source_id: i32,
}

impl Save<Show> for Show {
    type Error = DataError;

    /// Helper method to easily save/"sync" current state of self to the
    /// Database.
    fn save(&self) -> Result<Show, Self::Error> {
        let db = connection();
        let tempdb = db.get()?;

        self.save_changes::<Show>(&*tempdb).map_err(From::from)
    }
}

impl Show {
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

    /// Set the Show/Feed `link`.
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

    /// `Source` table foreign key.
    pub fn source_id(&self) -> i32 {
        self.source_id
    }
}

#[derive(Queryable, Debug, Clone)]
/// Diesel Model of the Show cover query.
/// Used for fetching information about a Show's cover.
pub struct ShowCoverModel {
    id: i32,
    title: String,
    image_uri: Option<String>,
}

impl From<Show> for ShowCoverModel {
    fn from(p: Show) -> ShowCoverModel {
        ShowCoverModel {
            id: p.id(),
            title: p.title,
            image_uri: p.image_uri,
        }
    }
}

impl ShowCoverModel {
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
