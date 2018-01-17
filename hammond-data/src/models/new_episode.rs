use diesel::prelude::*;
use diesel;
use schema::episode;

use rss;
use ammonia;
use rfc822_sanitizer::parse_from_rfc2822_with_fallback as parse_rfc822;

use dbqueries;
use errors::*;
use models::{Insert, Update};
use models::Episode;

use utils::{replace_extra_spaces, url_cleaner};
use parser;

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
    fn insert(&self, con: &SqliteConnection) -> QueryResult<usize> {
        use schema::episode::dsl::*;
        diesel::insert_into(episode).values(self).execute(&*con)
    }
}

impl Update for NewEpisode {
    fn update(&self, con: &SqliteConnection, episode_id: i32) -> QueryResult<usize> {
        use schema::episode::dsl::*;

        info!("Updating {:?}", self.title);
        diesel::update(episode.filter(rowid.eq(episode_id)))
            .set(self)
            .execute(&*con)
    }
}

impl NewEpisode {
    #[allow(dead_code)]
    /// Parses an `rss::Item` into a `NewEpisode` Struct.
    pub(crate) fn new(item: &rss::Item, podcast_id: i32) -> Result<Self> {
        NewEpisodeMinimal::new(item, podcast_id).map(|ep| ep.into_new_episode(item))
    }

    // TODO: Refactor into batch indexes instead.
    #[allow(dead_code)]
    pub(crate) fn into_episode(self, con: &SqliteConnection) -> Result<Episode> {
        self.index(con)?;
        Ok(dbqueries::get_episode_from_pk(
            con,
            &self.title,
            self.podcast_id,
        )?)
    }

    pub(crate) fn index(&self, con: &SqliteConnection) -> QueryResult<()> {
        let ep = dbqueries::get_episode_from_pk(con, &self.title, self.podcast_id);

        match ep {
            Ok(foo) => {
                if foo.podcast_id() != self.podcast_id {
                    error!("NEP pid: {}\nEP pid: {}", self.podcast_id, foo.podcast_id());
                };

                if foo.title() != self.title.as_str() || foo.epoch() != self.epoch
                    || foo.uri() != self.uri.as_ref().map(|s| s.as_str())
                    || foo.duration() != self.duration
                {
                    self.update(con, foo.rowid())?;
                }
            }
            Err(_) => {
                self.insert(con)?;
            }
        }
        Ok(())
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

impl NewEpisodeMinimal {
    #[allow(dead_code)]
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

        let date = parse_rfc822(
            // Default to rfc2822 represantation of epoch 0.
            item.pub_date().unwrap_or("Thu, 1 Jan 1970 00:00:00 +0000"),
        );
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

    #[allow(dead_code)]
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
