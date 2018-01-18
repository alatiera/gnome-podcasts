use diesel;
use diesel::prelude::*;

use ammonia;
use rss;

use models::{Insert, Update};
use models::Podcast;
use schema::podcast;

use database::connection;
use dbqueries;
use utils::{replace_extra_spaces, url_cleaner};

use errors::*;

#[derive(Insertable, AsChangeset)]
#[table_name = "podcast"]
#[derive(Debug, Clone, Default, Builder, PartialEq)]
#[builder(default)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub(crate) struct NewPodcast {
    title: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    source_id: i32,
}

impl Insert for NewPodcast {
    fn insert(&self) -> Result<()> {
        use schema::podcast::dsl::*;
        let db = connection();
        let con = db.get()?;

        diesel::insert_into(podcast)
            .values(self)
            .execute(&*con)
            .map(|_| ())
            .map_err(From::from)
    }
}

impl Update for NewPodcast {
    fn update(&self, podcast_id: i32) -> Result<()> {
        use schema::podcast::dsl::*;
        let db = connection();
        let con = db.get()?;

        info!("Updating {}", self.title);
        diesel::update(podcast.filter(id.eq(podcast_id)))
            .set(self)
            .execute(&*con)
            .map(|_| ())
            .map_err(From::from)
    }
}

impl NewPodcast {
    /// Parses a `rss::Channel` into a `NewPodcast` Struct.
    pub(crate) fn new(chan: &rss::Channel, source_id: i32) -> NewPodcast {
        let title = chan.title().trim();

        // Prefer itunes summary over rss.description since many feeds put html into
        // rss.description.
        let summary = chan.itunes_ext().map(|s| s.summary()).and_then(|s| s);
        let description = if let Some(sum) = summary {
            replace_extra_spaces(&ammonia::clean(sum))
        } else {
            replace_extra_spaces(&ammonia::clean(chan.description()))
        };

        let link = url_cleaner(chan.link());
        let x = chan.itunes_ext().map(|s| s.image());
        let image_uri = if let Some(img) = x {
            img.map(|s| s.to_owned())
        } else {
            chan.image().map(|foo| foo.url().to_owned())
        };

        NewPodcastBuilder::default()
            .title(title)
            .description(description)
            .link(link)
            .image_uri(image_uri)
            .source_id(source_id)
            .build()
            .unwrap()
    }

    // Look out for when tryinto lands into stable.
    pub(crate) fn into_podcast(self) -> Result<Podcast> {
        self.index()?;
        Ok(dbqueries::get_podcast_from_source_id(self.source_id)?)
    }

    pub(crate) fn index(&self) -> Result<()> {
        let pd = dbqueries::get_podcast_from_source_id(self.source_id);

        match pd {
            Ok(foo) => {
                if (foo.link() != self.link) || (foo.title() != self.title)
                    || (foo.image_uri() != self.image_uri.as_ref().map(|x| x.as_str()))
                {
                    info!("NewEpisode: {:?}\n OldEpisode: {:?}", self, foo);
                    self.update(foo.id())?;
                }
            }
            Err(_) => {
                self.insert()?;
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
// Ignore the following geters. They are used in unit tests mainly.
impl NewPodcast {
    pub(crate) fn source_id(&self) -> i32 {
        self.source_id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn link(&self) -> &str {
        &self.link
    }

    pub(crate) fn description(&self) -> &str {
        &self.description
    }

    pub(crate) fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_ref().map(|s| s.as_str())
    }
}
