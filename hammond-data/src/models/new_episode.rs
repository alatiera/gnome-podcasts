use diesel::prelude::*;

use diesel;
use schema::episode;

use ammonia;
use rfc822_sanitizer::parse_from_rfc2822_with_fallback as parse_rfc822;
use rss;

use database::connection;
use dbqueries;
use errors::*;
use models::{Episode, EpisodeMinimal, Index, Insert, Update};
use parser;
use utils::{replace_extra_spaces, url_cleaner};

#[derive(Insertable, AsChangeset)]
#[table_name = "episode"]
#[derive(Debug, Clone, Default, Builder, PartialEq)]
#[builder(default)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub(crate) struct NewEpisode {
    title: String,
    uri: Option<String>,
    description: Option<String>,
    length: Option<i32>,
    duration: Option<i32>,
    guid: Option<String>,
    epoch: i32,
    podcast_id: i32,
}

impl From<NewEpisodeMinimal> for NewEpisode {
    fn from(e: NewEpisodeMinimal) -> Self {
        NewEpisodeBuilder::default()
            .title(e.title)
            .uri(e.uri)
            .duration(e.duration)
            .epoch(e.epoch)
            .podcast_id(e.podcast_id)
            .guid(e.guid)
            .build()
            .unwrap()
    }
}

impl Insert for NewEpisode {
    fn insert(&self) -> Result<()> {
        use schema::episode::dsl::*;
        let db = connection();
        let con = db.get()?;

        info!("Indexing {:?}", self.title);
        diesel::insert_into(episode)
            .values(self)
            .execute(&*con)
            .map_err(From::from)
            .map(|_| ())
    }
}

impl Update for NewEpisode {
    fn update(&self, episode_id: i32) -> Result<()> {
        use schema::episode::dsl::*;
        let db = connection();
        let con = db.get()?;

        info!("Updating {:?}", self.title);
        diesel::update(episode.filter(rowid.eq(episode_id)))
            .set(self)
            .execute(&*con)
            .map_err(From::from)
            .map(|_| ())
    }
}

impl Index for NewEpisode {
    fn index(&self) -> Result<()> {
        let exists = dbqueries::episode_exists(self.title(), self.podcast_id())?;

        if exists {
            let other = dbqueries::get_episode_minimal_from_pk(self.title(), self.podcast_id())?;

            if self != &other {
                self.update(other.rowid())
            } else {
                Ok(())
            }
        } else {
            self.insert()
        }
    }
}

impl PartialEq<EpisodeMinimal> for NewEpisode {
    fn eq(&self, other: &EpisodeMinimal) -> bool {
        (self.title() == other.title()) && (self.uri() == other.uri())
            && (self.duration() == other.duration()) && (self.epoch() == other.epoch())
            && (self.guid() == other.guid())
    }
}

impl NewEpisode {
    /// Parses an `rss::Item` into a `NewEpisode` Struct.
    #[allow(dead_code)]
    pub(crate) fn new(item: &rss::Item, podcast_id: i32) -> Result<Self> {
        NewEpisodeMinimal::new(item, podcast_id).map(|ep| ep.into_new_episode(item))
    }

    #[allow(dead_code)]
    pub(crate) fn into_episode(self) -> Result<Episode> {
        self.index()?;
        dbqueries::get_episode_from_pk(&self.title, self.podcast_id)
    }
}

#[allow(dead_code)]
// Ignore the following getters. They are used in unit tests mainly.
impl NewEpisode {
    pub(crate) fn title(&self) -> &str {
        self.title.as_ref()
    }

    pub(crate) fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn guid(&self) -> Option<&str> {
        self.guid.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn epoch(&self) -> i32 {
        self.epoch
    }

    pub(crate) fn duration(&self) -> Option<i32> {
        self.duration
    }

    pub(crate) fn length(&self) -> Option<i32> {
        self.length
    }

    pub(crate) fn podcast_id(&self) -> i32 {
        self.podcast_id
    }
}

#[derive(Insertable, AsChangeset)]
#[table_name = "episode"]
#[derive(Debug, Clone, Default, Builder, PartialEq)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub(crate) struct NewEpisodeMinimal {
    title: String,
    uri: Option<String>,
    duration: Option<i32>,
    epoch: i32,
    guid: Option<String>,
    podcast_id: i32,
}

impl PartialEq<EpisodeMinimal> for NewEpisodeMinimal {
    fn eq(&self, other: &EpisodeMinimal) -> bool {
        (self.title() == other.title()) && (self.uri() == other.uri())
            && (self.duration() == other.duration()) && (self.epoch() == other.epoch())
            && (self.guid() == other.guid())
    }
}

impl NewEpisodeMinimal {
    pub(crate) fn new(item: &rss::Item, parent_id: i32) -> Result<Self> {
        if item.title().is_none() {
            bail!("No title specified for the item.")
        }

        let title = item.title().unwrap().trim().to_owned();
        let guid = item.guid().map(|s| s.value().trim().to_owned());

        let uri = if let Some(url) = item.enclosure().map(|s| url_cleaner(s.url())) {
            Some(url)
        } else if item.link().is_some() {
            item.link().map(|s| url_cleaner(s))
        } else {
            bail!("No url specified for the item.")
        };

        // Default to rfc2822 represantation of epoch 0.
        let date = parse_rfc822(item.pub_date().unwrap_or("Thu, 1 Jan 1970 00:00:00 +0000"));
        // Should treat information from the rss feeds as invalid by default.
        // Case: Thu, 05 Aug 2016 06:00:00 -0400 <-- Actually that was friday.
        let epoch = date.map(|x| x.timestamp() as i32).unwrap_or(0);

        let duration = parser::parse_itunes_duration(item);

        Ok(NewEpisodeMinimalBuilder::default()
            .title(title)
            .uri(uri)
            .duration(duration)
            .epoch(epoch)
            .guid(guid)
            .podcast_id(parent_id)
            .build()
            .unwrap())
    }

    pub(crate) fn into_new_episode(self, item: &rss::Item) -> NewEpisode {
        let length = || -> Option<i32> { item.enclosure().map(|x| x.length().parse().ok())? }();

        // Prefer itunes summary over rss.description since many feeds put html into
        // rss.description.
        let summary = item.itunes_ext().map(|s| s.summary()).and_then(|s| s);
        let description = if summary.is_some() {
            summary.map(|s| replace_extra_spaces(&ammonia::clean(s)))
        } else {
            item.description()
                .map(|s| replace_extra_spaces(&ammonia::clean(s)))
        };

        NewEpisodeBuilder::default()
            .title(self.title)
            .uri(self.uri)
            .duration(self.duration)
            .epoch(self.epoch)
            .podcast_id(self.podcast_id)
            .guid(self.guid)
            .length(length)
            .description(description)
            .build()
            .unwrap()
    }
}

// Ignore the following getters. They are used in unit tests mainly.
impl NewEpisodeMinimal {
    pub(crate) fn title(&self) -> &str {
        self.title.as_ref()
    }

    pub(crate) fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn guid(&self) -> Option<&str> {
        self.guid.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn duration(&self) -> Option<i32> {
        self.duration
    }

    pub(crate) fn epoch(&self) -> i32 {
        self.epoch
    }

    pub(crate) fn podcast_id(&self) -> i32 {
        self.podcast_id
    }
}
