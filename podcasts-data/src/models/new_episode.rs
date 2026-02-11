// new_episode.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::prelude::*;
use diesel::prelude::*;
use rfc822_sanitizer::parse_from_rfc2822_with_fallback as parse_rfc822;

use crate::database::connection;
use crate::dbqueries;
use crate::errors::DataError;
use crate::models::episode::EpisodeId;
use crate::models::{Episode, EpisodeMinimal, EpisodeModel, Index, Insert, ShowId, Update};
use crate::parser;
use crate::schema::episodes;
use crate::utils::url_cleaner;

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = episodes)]
#[derive(Debug, Clone, Default, Builder, PartialEq)]
#[builder(default)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub(crate) struct NewEpisode {
    title: String,
    uri: Option<String>,
    description: Option<String>,
    image_uri: Option<String>,
    length: Option<i32>,
    duration: Option<i32>,
    play_position: i32,
    guid: Option<String>,
    epoch: NaiveDateTime,
    show_id: ShowId,
}

impl From<NewEpisodeMinimal> for NewEpisode {
    fn from(e: NewEpisodeMinimal) -> Self {
        NewEpisodeBuilder::default()
            .title(e.title)
            .uri(e.uri)
            .image_uri(e.image_uri)
            .duration(e.duration)
            .epoch(e.epoch)
            .show_id(e.show_id)
            .guid(e.guid)
            .build()
            .unwrap()
    }
}

impl Insert<()> for NewEpisode {
    type Error = DataError;

    /// Should not be called directly, call index() instead.
    fn insert(&self) -> Result<(), DataError> {
        use crate::schema::episodes::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        info!("Inserting {:?}", self.title);
        diesel::insert_into(episodes)
            .values(self)
            .execute(&mut con)
            .map_err(From::from)
            .map(|_| ())
    }
}

impl Update<(), EpisodeId> for NewEpisode {
    type Error = DataError;

    fn update(&self, episode_id: EpisodeId) -> Result<(), DataError> {
        use crate::schema::episodes::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        info!("Updating {:?}", self.title);
        diesel::update(episodes.filter(id.eq(episode_id)))
            .set(self)
            .execute(&mut con)
            .map_err(From::from)
            .map(|_| ())
    }
}

impl Index<(), EpisodeId> for NewEpisode {
    type Error = DataError;

    // Does not update the episode description if it's the only thing that has
    // changed.
    fn index(&self) -> Result<(), DataError> {
        let exists = dbqueries::episode_exists(self.guid(), self.title(), self.show_id())?;

        if exists {
            let other = dbqueries::get_episode_minimal(self.guid(), self.title(), self.show_id())?;

            if self != &other {
                self.update(other.id())
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
        (self.title() == other.title())
            && (self.uri() == other.uri())
            && (self.image_uri() == other.image_uri())
            && (self.duration() == other.duration())
            && (self.epoch() == other.epoch())
            && (self.guid() == other.guid())
            && (self.show_id() == other.show_id())
    }
}

impl PartialEq<Episode> for NewEpisode {
    fn eq(&self, other: &Episode) -> bool {
        (self.title() == other.title())
            && (self.uri() == other.uri())
            && (self.image_uri() == other.image_uri())
            && (self.duration() == other.duration())
            && (self.play_position() == other.play_position())
            && (self.epoch() == other.epoch())
            && (self.guid() == other.guid())
            && (self.show_id() == other.show_id())
            && (self.description() == other.description())
            && (self.length() == other.length())
    }
}

impl NewEpisode {
    /// Parses an `rss::Item` into a `NewEpisode` Struct.
    #[allow(dead_code)]
    pub(crate) fn new(item: &rss::Item, show_id: ShowId) -> Result<Self, DataError> {
        NewEpisodeMinimal::new(item, show_id).map(|ep| ep.into_new_episode(item))
    }

    #[allow(dead_code)]
    pub(crate) fn to_episode(&self) -> Result<Episode, DataError> {
        self.index()?;

        dbqueries::get_episode(self.guid(), self.title(), self.show_id)
    }
}

// Ignore the following getters. They are used in unit tests mainly.
impl NewEpisode {
    pub(crate) fn title(&self) -> &str {
        self.title.as_ref()
    }

    pub(crate) fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }
    pub(crate) fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_deref()
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub(crate) fn guid(&self) -> Option<&str> {
        self.guid.as_deref()
    }

    pub(crate) fn epoch(&self) -> NaiveDateTime {
        self.epoch
    }

    pub(crate) fn duration(&self) -> Option<i32> {
        self.duration
    }

    pub(crate) fn play_position(&self) -> i32 {
        self.play_position
    }

    pub(crate) fn length(&self) -> Option<i32> {
        self.length
    }

    pub(crate) fn show_id(&self) -> ShowId {
        self.show_id
    }
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = episodes)]
#[derive(Debug, Clone, Builder, PartialEq)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub(crate) struct NewEpisodeMinimal {
    title: String,
    uri: Option<String>,
    image_uri: Option<String>,
    length: Option<i32>,
    duration: Option<i32>,
    #[builder(default = "0")]
    play_position: i32,
    epoch: NaiveDateTime,
    guid: Option<String>,
    show_id: ShowId,
}

impl PartialEq<EpisodeMinimal> for NewEpisodeMinimal {
    fn eq(&self, other: &EpisodeMinimal) -> bool {
        (self.title() == other.title())
            && (self.uri() == other.uri())
            && (self.image_uri() == other.image_uri())
            && (self.duration() == other.duration())
            && (self.epoch() == other.epoch())
            && (self.guid() == other.guid())
            && (self.show_id() == other.show_id())
    }
}

impl NewEpisodeMinimal {
    pub(crate) fn new(item: &rss::Item, parent_id: ShowId) -> Result<Self, DataError> {
        if item.title().is_none() {
            let err = DataError::ParseEpisodeError {
                reason: "No title specified for this Episode.".into(),
                parent_id,
            };

            return Err(err);
        }

        let title = item.title().unwrap().trim().to_owned();
        let guid = item.guid().map(|s| s.value().trim().to_owned());

        // Get the mime type, the `http` url and the length from the enclosure
        // http://www.rssboard.org/rss-specification#ltenclosuregtSubelementOfLtitemgt
        let enc = item.enclosure();

        // Get the url
        let uri = enc.map(|s| url_cleaner(s.url().trim()));

        let image = item
            .itunes_ext()
            .and_then(|i| i.image())
            .map(|s| s.to_owned());

        // Get the size of the content, it should be in bytes
        let length = enc.and_then(|x| x.length().parse().ok());

        // Default to rfc2822 representation of epoch 0.
        let date = parse_rfc822(item.pub_date().unwrap_or("Thu, 1 Jan 1970 00:00:00 +0000"));
        // Should treat information from the rss feeds as invalid by default.
        // Case: "Thu, 05 Aug 2016 06:00:00 -0400" <-- Actually that was friday.
        let epoch = date
            .map(|x| DateTime::<Utc>::from(x).naive_utc())
            .unwrap_or_default();

        let duration = parser::parse_itunes_duration(item.itunes_ext());

        NewEpisodeMinimalBuilder::default()
            .title(title)
            .uri(uri)
            .image_uri(image)
            .length(length)
            .duration(duration)
            .epoch(epoch)
            .guid(guid)
            .show_id(parent_id)
            .build()
            .map_err(|err| DataError::BuilderError(format!("{err}")))
    }

    // TODO: TryInto is stabilizing in rustc v1.26!
    // ^ Jokes on you past self!
    pub(crate) fn into_new_episode(self, item: &rss::Item) -> NewEpisode {
        let description = item.content().or(item.description()).map(|s| {
            let sanitized_html = ammonia::Builder::new()
                // Remove `rel` attributes from `<a>` tags
                .link_rel(None)
                .clean(s.trim())
                .to_string();
            sanitized_html
        });

        NewEpisodeBuilder::default()
            .title(self.title)
            .uri(self.uri)
            .image_uri(self.image_uri)
            .duration(self.duration)
            .epoch(self.epoch)
            .show_id(self.show_id)
            .guid(self.guid)
            .length(self.length)
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
        self.uri.as_deref()
    }

    pub(crate) fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_deref()
    }

    pub(crate) fn guid(&self) -> Option<&str> {
        self.guid.as_deref()
    }

    pub(crate) fn duration(&self) -> Option<i32> {
        self.duration
    }

    pub(crate) fn epoch(&self) -> NaiveDateTime {
        self.epoch
    }

    pub(crate) fn show_id(&self) -> ShowId {
        self.show_id
    }
}

#[cfg(test)]
mod tests {
    use crate::database::reset_db;
    use crate::dbqueries;
    use crate::models::new_episode::{NewEpisodeMinimal, NewEpisodeMinimalBuilder};
    use crate::models::*;
    use anyhow::Result;
    use chrono::prelude::*;

    use rss::Channel;

    use std::fs::File;
    use std::io::BufReader;
    use std::sync::LazyLock;

    /// randomly chosen
    const TEST_SHOW_ID: ShowId = ShowId(42);

    // TODO: Add tests for other feeds too.
    // Especially if you find an *interesting* generated feed.

    // Known prebuilt expected objects.
    static EXPECTED_MINIMAL_INTERCEPTED_1: LazyLock<NewEpisodeMinimal> = LazyLock::new(|| {
        NewEpisodeMinimalBuilder::default()
            .title("The Super Bowl of Racism")
            .uri(Some(String::from(
                "http://traffic.megaphone.fm/PPY6458293736.mp3",
            )))
            .image_uri(None)
            .guid(Some(String::from("7df4070a-9832-11e7-adac-cb37b05d5e24")))
            .epoch(
                DateTime::<Utc>::from_timestamp(1505296800, 0)
                    .unwrap()
                    .naive_utc(),
            )
            .length(Some(66738886))
            .duration(Some(4171))
            .show_id(TEST_SHOW_ID)
            .build()
            .unwrap()
    });
    static EXPECTED_MINIMAL_INTERCEPTED_2: LazyLock<NewEpisodeMinimal> = LazyLock::new(|| {
        NewEpisodeMinimalBuilder::default()
            .title("Atlas Golfed — U.S.-Backed Think Tanks Target Latin America")
            .uri(Some(String::from(
                "http://traffic.megaphone.fm/FL5331443769.mp3",
            )))
            .image_uri(None)
            .guid(Some(String::from("7c207a24-e33f-11e6-9438-eb45dcf36a1d")))
            .epoch(
                DateTime::<Utc>::from_timestamp(1502272800, 0)
                    .unwrap()
                    .naive_utc(),
            )
            .length(Some(67527575))
            .duration(Some(4415))
            .show_id(TEST_SHOW_ID)
            .build()
            .unwrap()
    });
    static EXPECTED_INTERCEPTED_1: LazyLock<NewEpisode> = LazyLock::new(|| {
        let descr = "NSA whistleblower Edward Snowden discusses the massive Equifax data \
                         breach and allegations of Russian interference in the US election. \
                         Commentator Shaun King explains his call for a boycott of the NFL and \
                         talks about his campaign to bring violent neo-Nazis to justice. Rapper \
                         Open Mike Eagle performs.";

        NewEpisodeBuilder::default()
            .title("The Super Bowl of Racism")
            .uri(Some(String::from(
                "http://traffic.megaphone.fm/PPY6458293736.mp3",
            )))
            .image_uri(None)
            .description(Some(String::from(descr)))
            .guid(Some(String::from("7df4070a-9832-11e7-adac-cb37b05d5e24")))
            .length(Some(66738886))
            .epoch(
                DateTime::<Utc>::from_timestamp(1505296800, 0)
                    .unwrap()
                    .naive_utc(),
            )
            .duration(Some(4171))
            .show_id(TEST_SHOW_ID)
            .build()
            .unwrap()
    });
    static EXPECTED_INTERCEPTED_2: LazyLock<NewEpisode> = LazyLock::new(|| {
        let descr = "This week on Intercepted: Jeremy gives an update on the aftermath of \
                         Blackwater’s 2007 massacre of Iraqi civilians. Intercept reporter Lee \
                         Fang lays out how a network of libertarian think tanks called the Atlas \
                         Network is insidiously shaping political infrastructure in Latin \
                         America. We speak with attorney and former Hugo Chavez adviser Eva \
                         Golinger about the Venezuela\'s political turmoil.And we hear Claudia \
                         Lizardo of the Caracas-based band, La Pequeña Revancha, talk about her \
                         music and hopes for Venezuela.";

        NewEpisodeBuilder::default()
            .title("Atlas Golfed — U.S.-Backed Think Tanks Target Latin America")
            .uri(Some(String::from(
                "http://traffic.megaphone.fm/FL5331443769.mp3",
            )))
            .image_uri(None)
            .description(Some(String::from(descr)))
            .guid(Some(String::from("7c207a24-e33f-11e6-9438-eb45dcf36a1d")))
            .length(Some(67527575))
            .epoch(
                DateTime::<Utc>::from_timestamp(1502272800, 0)
                    .unwrap()
                    .naive_utc(),
            )
            .duration(Some(4415))
            .show_id(TEST_SHOW_ID)
            .build()
            .unwrap()
    });
    static UPDATED_DURATION_INTERCEPTED_1: LazyLock<NewEpisode> = LazyLock::new(|| {
        NewEpisodeBuilder::default()
            .title("The Super Bowl of Racism")
            .uri(Some(String::from(
                "http://traffic.megaphone.fm/PPY6458293736.mp3",
            )))
            .image_uri(None)
            .description(Some(String::from("New description")))
            .guid(Some(String::from("7df4070a-9832-11e7-adac-cb37b05d5e24")))
            .length(Some(66738886))
            .epoch(
                DateTime::<Utc>::from_timestamp(1505296800, 0)
                    .unwrap()
                    .naive_utc(),
            )
            .duration(Some(424242))
            .show_id(TEST_SHOW_ID)
            .build()
            .unwrap()
    });
    static EXPECTED_MINIMAL_LUP_1: LazyLock<NewEpisodeMinimal> = LazyLock::new(|| {
        NewEpisodeMinimalBuilder::default()
            .title("Hacking Devices with Kali Linux | LUP 214")
            .uri(Some(String::from(
                "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0214.mp3",
            )))
            .image_uri(None)
            .guid(Some(String::from("78A682B4-73E8-47B8-88C0-1BE62DD4EF9D")))
            .length(Some(46479789))
            .epoch(
                DateTime::<Utc>::from_timestamp(1505280282, 0)
                    .unwrap()
                    .naive_utc(),
            )
            .duration(Some(5733))
            .show_id(TEST_SHOW_ID)
            .build()
            .unwrap()
    });
    static EXPECTED_MINIMAL_LUP_2: LazyLock<NewEpisodeMinimal> = LazyLock::new(|| {
        NewEpisodeMinimalBuilder::default()
            .title("Gnome Does it Again | LUP 213")
            .uri(Some(String::from(
                "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0213.mp3",
            )))
            .image_uri(None)
            .guid(Some(String::from("1CE57548-B36C-4F14-832A-5D5E0A24E35B")))
            .epoch(
                DateTime::<Utc>::from_timestamp(1504670247, 0)
                    .unwrap()
                    .naive_utc(),
            )
            .length(Some(36544272))
            .duration(Some(4491))
            .show_id(TEST_SHOW_ID)
            .build()
            .unwrap()
    });
    static EXPECTED_LUP_1: LazyLock<NewEpisode> = LazyLock::new(|| {
        let descr = "Audit your network with a couple of easy commands on Kali Linux. Chris \
                         decides to blow off a little steam by attacking his IoT devices, Wes has \
                         the scope on Equifax blaming open source &amp; the Beard just saved the \
                         show. It’s a really packed episode!";

        NewEpisodeBuilder::default()
            .title("Hacking Devices with Kali Linux | LUP 214")
            .uri(Some(String::from(
                "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0214.mp3",
            )))
            .image_uri(None)
            .description(Some(String::from(descr)))
            .guid(Some(String::from("78A682B4-73E8-47B8-88C0-1BE62DD4EF9D")))
            .length(Some(46479789))
            .epoch(
                DateTime::<Utc>::from_timestamp(1505280282, 0)
                    .unwrap()
                    .naive_utc(),
            )
            .duration(Some(5733))
            .show_id(TEST_SHOW_ID)
            .build()
            .unwrap()
    });
    static EXPECTED_LUP_2: LazyLock<NewEpisode> = LazyLock::new(|| {
        let descr = "<p>The Gnome project is about to solve one of our audience's biggest Wayland’s \
                 concerns. But as the project takes on a new level of relevance, decisions for \
                 the next version of Gnome have us worried about the future.</p>\n\n<p>Plus we \
                 chat with Wimpy about the Ubuntu Rally in NYC, Microsoft’s sneaky move to turn \
                 Windows 10 into the “ULTIMATE LINUX RUNTIME”, community news &amp; more!</p>";

        NewEpisodeBuilder::default()
            .title("Gnome Does it Again | LUP 213")
            .uri(Some(String::from(
                "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0213.mp3",
            )))
            .image_uri(None)
            .description(Some(String::from(descr)))
            .guid(Some(String::from("1CE57548-B36C-4F14-832A-5D5E0A24E35B")))
            .length(Some(36544272))
            .epoch(
                DateTime::<Utc>::from_timestamp(1504670247, 0)
                    .unwrap()
                    .naive_utc(),
            )
            .duration(Some(4491))
            .show_id(TEST_SHOW_ID)
            .build()
            .unwrap()
    });

    static EXPECTED_NDR_1: LazyLock<NewEpisode> = LazyLock::new(|| {
        let descr = "<p>Die aktuellen Meldungen aus der NDR Info Nachrichtenredaktion.</p>";

        NewEpisodeBuilder::default()
            .title("Nachrichten")
            .uri(Some(String::from(
                "https://mediandr-a.akamaihd.net/download/podcasts/podcast4450/AU-20240313-2303-4300.mp3",
            )))
            .description(Some(String::from(descr)))
            .guid(Some(String::from("AU-20240313-2303-4300-A")))
            .length(None)
            .epoch(DateTime::<Utc>::from_timestamp(1710367140, 0).unwrap().naive_utc())
            .duration(Some(202))
            .show_id(TEST_SHOW_ID)
            .image_uri(Some("https://www.ndr.de/nachrichten/info/nachrichten660_v-quadratl.jpg".to_string()))
            .build()
            .unwrap()
    });

    static EXPECTED_NDR_2: LazyLock<NewEpisode> = LazyLock::new(|| {
        let descr = "<p>Die aktuellen Meldungen aus der NDR Info Nachrichtenredaktion.</p>";

        NewEpisodeBuilder::default()
            .title("Nachrichten")
            .uri(Some(String::from(
                "https://mediandr-a.akamaihd.net/download/podcasts/podcast4450/AU-20240314-1705-4100.mp3",
            )))
            .description(Some(String::from(descr)))
            .guid(Some(String::from("AU-20240314-1705-4100-A")))
            .length(None)
            .epoch(DateTime::<Utc>::from_timestamp(1710431940, 0).unwrap().naive_utc())
            .duration(Some(300))
            .show_id(TEST_SHOW_ID)
            .image_uri(Some("https://www.ndr.de/nachrichten/info/nachrichten660_v-quadratl.jpg".to_string()))
            .build()
            .unwrap()
    });

    static EXPECTED_NDR_3: LazyLock<NewEpisode> = LazyLock::new(|| {
        let descr = "<p>Die aktuellen Meldungen aus der NDR Info Nachrichtenredaktion.</p>";

        NewEpisodeBuilder::default()
            .title("TITLE_UPDATED")
            .uri(Some(String::from(
                "https://mediandr-a.akamaihd.net/download/podcasts/podcast4450/AU-20240314-1705-4100.mp3",
            )))
            .description(Some(String::from(descr)))
            .guid(Some(String::from("AU-20240314-1705-4100-A")))
            .length(None)
            .epoch(DateTime::<Utc>::from_timestamp(2000000000, 0).unwrap().naive_utc())
            .duration(Some(300))
            .show_id(TEST_SHOW_ID)
            .image_uri(Some("https://www.ndr.de/nachrichten/info/nachrichten660_v-quadratl.jpg".to_string()))
            .build()
            .unwrap()
    });

    #[test]
    fn test_new_episode_minimal_intercepted() -> Result<()> {
        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(14).unwrap();
        let ep = NewEpisodeMinimal::new(episode, TEST_SHOW_ID)?;
        assert_eq!(ep, *EXPECTED_MINIMAL_INTERCEPTED_1);

        let episode = channel.items().iter().nth(15).unwrap();
        let ep = NewEpisodeMinimal::new(episode, TEST_SHOW_ID)?;
        assert_eq!(ep, *EXPECTED_MINIMAL_INTERCEPTED_2);
        Ok(())
    }

    #[test]
    fn test_new_episode_intercepted() -> Result<()> {
        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(14).unwrap();
        let ep = NewEpisode::new(episode, TEST_SHOW_ID)?;
        assert_eq!(ep, *EXPECTED_INTERCEPTED_1);

        let episode = channel.items().iter().nth(15).unwrap();
        let ep = NewEpisode::new(episode, TEST_SHOW_ID)?;

        assert_eq!(ep, *EXPECTED_INTERCEPTED_2);
        Ok(())
    }

    #[test]
    fn test_new_episode_minimal_lup() -> Result<()> {
        let file = File::open("tests/feeds/2018-01-20-LinuxUnplugged.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(18).unwrap();
        let ep = NewEpisodeMinimal::new(episode, TEST_SHOW_ID)?;
        assert_eq!(ep, *EXPECTED_MINIMAL_LUP_1);

        let episode = channel.items().iter().nth(19).unwrap();
        let ep = NewEpisodeMinimal::new(episode, TEST_SHOW_ID)?;
        assert_eq!(ep, *EXPECTED_MINIMAL_LUP_2);
        Ok(())
    }

    #[test]
    fn test_new_episode_lup() -> Result<()> {
        let file = File::open("tests/feeds/2018-01-20-LinuxUnplugged.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(18).unwrap();
        let ep = NewEpisode::new(episode, TEST_SHOW_ID)?;
        assert_eq!(ep, *EXPECTED_LUP_1);

        let episode = channel.items().iter().nth(19).unwrap();
        let ep = NewEpisode::new(episode, TEST_SHOW_ID)?;
        assert_eq!(ep, *EXPECTED_LUP_2);
        Ok(())
    }

    #[test]
    fn test_minimal_into_new_episode() -> Result<()> {
        let _tempfile = reset_db()?;

        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let item = channel.items().iter().nth(14).unwrap();
        let ep = EXPECTED_MINIMAL_INTERCEPTED_1
            .clone()
            .into_new_episode(item);
        assert_eq!(ep, *EXPECTED_INTERCEPTED_1);

        let item = channel.items().iter().nth(15).unwrap();
        let ep = EXPECTED_MINIMAL_INTERCEPTED_2
            .clone()
            .into_new_episode(item);
        assert_eq!(ep, *EXPECTED_INTERCEPTED_2);
        Ok(())
    }

    #[test]
    fn test_new_episode_insert() -> Result<()> {
        let _tempfile = reset_db()?;

        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(14).unwrap();
        let new_ep = NewEpisode::new(episode, TEST_SHOW_ID)?;
        new_ep.index()?;
        let ep = dbqueries::get_episode(new_ep.guid(), new_ep.title(), new_ep.show_id())?;

        assert_eq!(new_ep, ep);
        assert_eq!(&new_ep, &*EXPECTED_INTERCEPTED_1);
        assert_eq!(&*EXPECTED_INTERCEPTED_1, &ep);

        let episode = channel.items().iter().nth(15).unwrap();
        let new_ep = NewEpisode::new(episode, TEST_SHOW_ID)?;
        new_ep.index()?;
        let ep = dbqueries::get_episode(new_ep.guid(), new_ep.title(), new_ep.show_id())?;

        assert_eq!(new_ep, ep);
        assert_eq!(&new_ep, &*EXPECTED_INTERCEPTED_2);
        assert_eq!(&*EXPECTED_INTERCEPTED_2, &ep);
        Ok(())
    }

    #[test]
    fn test_new_episode_update() -> Result<()> {
        let _tempfile = reset_db()?;
        let old = EXPECTED_INTERCEPTED_1.clone().to_episode()?;

        let updated = &*UPDATED_DURATION_INTERCEPTED_1;
        updated.update(old.id())?;
        let new = dbqueries::get_episode(old.guid(), old.title(), old.show_id())?;

        // Assert that updating does not change the id and show_id
        assert_ne!(old, new);
        assert_eq!(old.id(), new.id());
        assert_eq!(old.show_id(), new.show_id());

        assert_eq!(updated, &new);
        assert_ne!(updated, &old);
        Ok(())
    }

    #[test]
    fn test_new_episode_index() -> Result<()> {
        let _tempfile = reset_db()?;
        let expected = &*EXPECTED_INTERCEPTED_1;

        // First insert
        assert!(expected.index().is_ok());
        // Second identical, This should take the early return path
        assert!(expected.index().is_ok());
        // Get the episode
        let old = dbqueries::get_episode(expected.guid(), expected.title(), expected.show_id())?;
        // Assert that NewPodcast is equal to the Indexed one
        assert_eq!(*expected, old);

        let updated = &*UPDATED_DURATION_INTERCEPTED_1;

        // Update the podcast
        assert!(updated.index().is_ok());
        // Get the new Podcast
        let new = dbqueries::get_episode(expected.guid(), expected.title(), expected.show_id())?;
        // Assert it's diff from the old one.
        assert_ne!(new, old);
        assert_eq!(*updated, new);
        assert_eq!(new.id(), old.id());
        assert_eq!(new.show_id(), old.show_id());
        Ok(())
    }

    #[test]
    fn test_new_episode_to_episode() -> Result<()> {
        let expected = &*EXPECTED_INTERCEPTED_1;

        // Assert insert() produces the same result that you would get with to_podcast()
        let _tempfile = reset_db()?;
        expected.index()?;
        let old = dbqueries::get_episode(expected.guid(), expected.title(), expected.show_id())?;
        let ep = expected.to_episode()?;
        assert_eq!(old, ep);

        // Same as above, diff order
        let _tempfile = reset_db()?;
        let ep = expected.to_episode()?;

        // did not make a new insert, updated
        expected.index()?;
        assert_eq!(dbqueries::get_episodes()?.len(), 1);

        let old = dbqueries::get_episode(expected.guid(), expected.title(), expected.show_id())?;
        assert_eq!(old, ep);
        Ok(())
    }

    // https://gitlab.gnome.org/World/podcasts/-/issues/216
    // new episode is imported, always same title, different guid
    #[test]
    fn test_feed_ndr() -> Result<()> {
        let _tempfile = reset_db()?;

        let file = File::open("tests/feeds/2024-03-13-ndr.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(1).unwrap();
        let new_ep = NewEpisode::new(episode, TEST_SHOW_ID)?;
        new_ep.index()?;
        let ep = dbqueries::get_episode(new_ep.guid(), new_ep.title(), new_ep.show_id())?;

        assert_eq!(new_ep, ep);
        assert_eq!(&new_ep, &*EXPECTED_NDR_1);
        assert_eq!(&*EXPECTED_NDR_1, &ep);

        let file = File::open("tests/feeds/2024-03-14-ndr.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(1).unwrap();
        let new_ep = NewEpisode::new(episode, TEST_SHOW_ID)?;
        new_ep.index()?;
        let ep = dbqueries::get_episode(new_ep.guid(), new_ep.title(), new_ep.show_id())?;

        assert_eq!(new_ep, ep);
        assert_eq!(&new_ep, &*EXPECTED_NDR_2);
        assert_eq!(&*EXPECTED_NDR_2, &ep);

        let all_eps = dbqueries::get_episodes()?;
        assert_eq!(2, all_eps.len());

        // update one of the ep's title and epoch
        let new_ep = &*EXPECTED_NDR_3;
        new_ep.index()?;

        // https://gitlab.gnome.org/World/podcasts/-/issues/151
        // Title update
        let ep = dbqueries::get_episode(new_ep.guid(), new_ep.title(), new_ep.show_id())?;
        assert_eq!(new_ep, &ep);
        assert_eq!(new_ep, &*EXPECTED_NDR_3);
        assert_eq!(&*EXPECTED_NDR_3, &ep);
        assert_eq!(
            DateTime::<Utc>::from_timestamp(2000000000, 0)
                .unwrap()
                .naive_utc(),
            ep.epoch()
        );
        assert_eq!("TITLE_UPDATED", ep.title());

        let all_eps = dbqueries::get_episodes()?;
        assert_eq!(2, all_eps.len());

        Ok(())
    }
}
