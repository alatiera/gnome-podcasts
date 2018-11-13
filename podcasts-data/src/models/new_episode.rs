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

use ammonia;
use diesel;
use diesel::prelude::*;
use rfc822_sanitizer::parse_from_rfc2822_with_fallback as parse_rfc822;
use rss;

use database::connection;
use dbqueries;
use errors::DataError;
use models::{Episode, EpisodeMinimal, Index, Insert, Update};
use parser;
use schema::episodes;
use utils::url_cleaner;

#[derive(Insertable, AsChangeset)]
#[table_name = "episodes"]
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
    show_id: i32,
}

impl From<NewEpisodeMinimal> for NewEpisode {
    fn from(e: NewEpisodeMinimal) -> Self {
        NewEpisodeBuilder::default()
            .title(e.title)
            .uri(e.uri)
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

    fn insert(&self) -> Result<(), DataError> {
        use schema::episodes::dsl::*;
        let db = connection();
        let con = db.get()?;

        info!("Inserting {:?}", self.title);
        diesel::insert_into(episodes)
            .values(self)
            .execute(&con)
            .map_err(From::from)
            .map(|_| ())
    }
}

impl Update<()> for NewEpisode {
    type Error = DataError;

    fn update(&self, episode_id: i32) -> Result<(), DataError> {
        use schema::episodes::dsl::*;
        let db = connection();
        let con = db.get()?;

        info!("Updating {:?}", self.title);
        diesel::update(episodes.filter(rowid.eq(episode_id)))
            .set(self)
            .execute(&con)
            .map_err(From::from)
            .map(|_| ())
    }
}

impl Index<()> for NewEpisode {
    type Error = DataError;

    // Does not update the episode description if it's the only thing that has
    // changed.
    fn index(&self) -> Result<(), DataError> {
        let exists = dbqueries::episode_exists(self.title(), self.show_id())?;

        if exists {
            let other = dbqueries::get_episode_minimal_from_pk(self.title(), self.show_id())?;

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
        (self.title() == other.title())
            && (self.uri() == other.uri())
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
            && (self.duration() == other.duration())
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
    pub(crate) fn new(item: &rss::Item, show_id: i32) -> Result<Self, DataError> {
        NewEpisodeMinimal::new(item, show_id).map(|ep| ep.into_new_episode(item))
    }

    #[allow(dead_code)]
    pub(crate) fn to_episode(&self) -> Result<Episode, DataError> {
        self.index()?;
        dbqueries::get_episode_from_pk(&self.title, self.show_id).map_err(From::from)
    }
}

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

    pub(crate) fn show_id(&self) -> i32 {
        self.show_id
    }
}

#[derive(Insertable, AsChangeset)]
#[table_name = "episodes"]
#[derive(Debug, Clone, Builder, PartialEq)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub(crate) struct NewEpisodeMinimal {
    title: String,
    uri: Option<String>,
    length: Option<i32>,
    duration: Option<i32>,
    epoch: i32,
    guid: Option<String>,
    show_id: i32,
}

impl PartialEq<EpisodeMinimal> for NewEpisodeMinimal {
    fn eq(&self, other: &EpisodeMinimal) -> bool {
        (self.title() == other.title())
            && (self.uri() == other.uri())
            && (self.duration() == other.duration())
            && (self.epoch() == other.epoch())
            && (self.guid() == other.guid())
            && (self.show_id() == other.show_id())
    }
}

impl NewEpisodeMinimal {
    pub(crate) fn new(item: &rss::Item, parent_id: i32) -> Result<Self, DataError> {
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
        let uri = enc
            .map(|s| url_cleaner(s.url().trim()))
            // Fallback to Rss.Item.link if enclosure is None.
            .or_else(|| item.link().map(|s| url_cleaner(s.trim())));

        // Get the size of the content, it should be in bytes
        let length = enc.and_then(|x| x.length().parse().ok());

        // If url is still None return an Error as this behaviour is not
        // compliant with the RSS Spec.
        if uri.is_none() {
            let err = DataError::ParseEpisodeError {
                reason: "No url specified for the item.".into(),
                parent_id,
            };

            return Err(err);
        };

        // Default to rfc2822 represantation of epoch 0.
        let date = parse_rfc822(item.pub_date().unwrap_or("Thu, 1 Jan 1970 00:00:00 +0000"));
        // Should treat information from the rss feeds as invalid by default.
        // Case: "Thu, 05 Aug 2016 06:00:00 -0400" <-- Actually that was friday.
        let epoch = date.map(|x| x.timestamp() as i32).unwrap_or(0);

        let duration = parser::parse_itunes_duration(item.itunes_ext());

        NewEpisodeMinimalBuilder::default()
            .title(title)
            .uri(uri)
            .length(length)
            .duration(duration)
            .epoch(epoch)
            .guid(guid)
            .show_id(parent_id)
            .build()
            .map_err(From::from)
    }

    // TODO: TryInto is stabilizing in rustc v1.26!
    // ^ Jokes on you past self!
    pub(crate) fn into_new_episode(self, item: &rss::Item) -> NewEpisode {
        let description = item.description().and_then(|s| {
            let sanitized_html = ammonia::Builder::new()
                // Remove `rel` attributes from `<a>` tags
                .link_rel(None)
                .clean(s.trim())
                .to_string();
            Some(sanitized_html)
        });

        NewEpisodeBuilder::default()
            .title(self.title)
            .uri(self.uri)
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

    pub(crate) fn show_id(&self) -> i32 {
        self.show_id
    }
}

#[cfg(test)]
mod tests {
    use database::truncate_db;
    use dbqueries;
    use failure::Error;
    use models::new_episode::{NewEpisodeMinimal, NewEpisodeMinimalBuilder};
    use models::*;

    use rss::Channel;

    use std::fs::File;
    use std::io::BufReader;

    // TODO: Add tests for other feeds too.
    // Especially if you find an *intresting* generated feed.

    // Known prebuilt expected objects.
    lazy_static! {
        static ref EXPECTED_MINIMAL_INTERCEPTED_1: NewEpisodeMinimal = {
            NewEpisodeMinimalBuilder::default()
                .title("The Super Bowl of Racism")
                .uri(Some(String::from(
                    "http://traffic.megaphone.fm/PPY6458293736.mp3",
                )))
                .guid(Some(String::from("7df4070a-9832-11e7-adac-cb37b05d5e24")))
                .epoch(1505296800)
                .length(Some(66738886))
                .duration(Some(4171))
                .show_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_MINIMAL_INTERCEPTED_2: NewEpisodeMinimal = {
            NewEpisodeMinimalBuilder::default()
                .title("Atlas Golfed — U.S.-Backed Think Tanks Target Latin America")
                .uri(Some(String::from(
                    "http://traffic.megaphone.fm/FL5331443769.mp3",
                )))
                .guid(Some(String::from("7c207a24-e33f-11e6-9438-eb45dcf36a1d")))
                .epoch(1502272800)
                .length(Some(67527575))
                .duration(Some(4415))
                .show_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_INTERCEPTED_1: NewEpisode = {
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
                .description(Some(String::from(descr)))
                .guid(Some(String::from("7df4070a-9832-11e7-adac-cb37b05d5e24")))
                .length(Some(66738886))
                .epoch(1505296800)
                .duration(Some(4171))
                .show_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_INTERCEPTED_2: NewEpisode = {
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
                .description(Some(String::from(descr)))
                .guid(Some(String::from("7c207a24-e33f-11e6-9438-eb45dcf36a1d")))
                .length(Some(67527575))
                .epoch(1502272800)
                .duration(Some(4415))
                .show_id(42)
                .build()
                .unwrap()
        };
        static ref UPDATED_DURATION_INTERCEPTED_1: NewEpisode = {
            NewEpisodeBuilder::default()
                .title("The Super Bowl of Racism")
                .uri(Some(String::from(
                    "http://traffic.megaphone.fm/PPY6458293736.mp3",
                )))
                .description(Some(String::from("New description")))
                .guid(Some(String::from("7df4070a-9832-11e7-adac-cb37b05d5e24")))
                .length(Some(66738886))
                .epoch(1505296800)
                .duration(Some(424242))
                .show_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_MINIMAL_LUP_1: NewEpisodeMinimal = {
            NewEpisodeMinimalBuilder::default()
                .title("Hacking Devices with Kali Linux | LUP 214")
                .uri(Some(String::from(
                    "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0214.mp3",
                )))
                .guid(Some(String::from("78A682B4-73E8-47B8-88C0-1BE62DD4EF9D")))
                .length(Some(46479789))
                .epoch(1505280282)
                .duration(Some(5733))
                .show_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_MINIMAL_LUP_2: NewEpisodeMinimal = {
            NewEpisodeMinimalBuilder::default()
                .title("Gnome Does it Again | LUP 213")
                .uri(Some(String::from(
                    "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0213.mp3",
                )))
                .guid(Some(String::from("1CE57548-B36C-4F14-832A-5D5E0A24E35B")))
                .epoch(1504670247)
                .length(Some(36544272))
                .duration(Some(4491))
                .show_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_LUP_1: NewEpisode = {
            let descr = "Audit your network with a couple of easy commands on Kali Linux. Chris \
                         decides to blow off a little steam by attacking his IoT devices, Wes has \
                         the scope on Equifax blaming open source &amp; the Beard just saved the \
                         show. It’s a really packed episode!";

            NewEpisodeBuilder::default()
                .title("Hacking Devices with Kali Linux | LUP 214")
                .uri(Some(String::from(
                    "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0214.mp3",
                )))
                .description(Some(String::from(descr)))
                .guid(Some(String::from("78A682B4-73E8-47B8-88C0-1BE62DD4EF9D")))
                .length(Some(46479789))
                .epoch(1505280282)
                .duration(Some(5733))
                .show_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_LUP_2: NewEpisode = {
            let descr =
                "<p>The Gnome project is about to solve one of our audience's biggest Wayland’s \
                 concerns. But as the project takes on a new level of relevance, decisions for \
                 the next version of Gnome have us worried about the future.</p>\n\n<p>Plus we \
                 chat with Wimpy about the Ubuntu Rally in NYC, Microsoft’s sneaky move to turn \
                 Windows 10 into the “ULTIMATE LINUX RUNTIME”, community news &amp; more!</p>";

            NewEpisodeBuilder::default()
                .title("Gnome Does it Again | LUP 213")
                .uri(Some(String::from(
                    "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0213.mp3",
                )))
                .description(Some(String::from(descr)))
                .guid(Some(String::from("1CE57548-B36C-4F14-832A-5D5E0A24E35B")))
                .length(Some(36544272))
                .epoch(1504670247)
                .duration(Some(4491))
                .show_id(42)
                .build()
                .unwrap()
        };
    }

    #[test]
    fn test_new_episode_minimal_intercepted() -> Result<(), Error> {
        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(14).unwrap();
        let ep = NewEpisodeMinimal::new(&episode, 42)?;
        assert_eq!(ep, *EXPECTED_MINIMAL_INTERCEPTED_1);

        let episode = channel.items().iter().nth(15).unwrap();
        let ep = NewEpisodeMinimal::new(&episode, 42)?;
        assert_eq!(ep, *EXPECTED_MINIMAL_INTERCEPTED_2);
        Ok(())
    }

    #[test]
    fn test_new_episode_intercepted() -> Result<(), Error> {
        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(14).unwrap();
        let ep = NewEpisode::new(&episode, 42)?;
        assert_eq!(ep, *EXPECTED_INTERCEPTED_1);

        let episode = channel.items().iter().nth(15).unwrap();
        let ep = NewEpisode::new(&episode, 42)?;

        assert_eq!(ep, *EXPECTED_INTERCEPTED_2);
        Ok(())
    }

    #[test]
    fn test_new_episode_minimal_lup() -> Result<(), Error> {
        let file = File::open("tests/feeds/2018-01-20-LinuxUnplugged.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(18).unwrap();
        let ep = NewEpisodeMinimal::new(&episode, 42)?;
        assert_eq!(ep, *EXPECTED_MINIMAL_LUP_1);

        let episode = channel.items().iter().nth(19).unwrap();
        let ep = NewEpisodeMinimal::new(&episode, 42)?;
        assert_eq!(ep, *EXPECTED_MINIMAL_LUP_2);
        Ok(())
    }

    #[test]
    fn test_new_episode_lup() -> Result<(), Error> {
        let file = File::open("tests/feeds/2018-01-20-LinuxUnplugged.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(18).unwrap();
        let ep = NewEpisode::new(&episode, 42)?;
        assert_eq!(ep, *EXPECTED_LUP_1);

        let episode = channel.items().iter().nth(19).unwrap();
        let ep = NewEpisode::new(&episode, 42)?;
        assert_eq!(ep, *EXPECTED_LUP_2);
        Ok(())
    }

    #[test]
    fn test_minimal_into_new_episode() -> Result<(), Error> {
        truncate_db()?;

        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let item = channel.items().iter().nth(14).unwrap();
        let ep = EXPECTED_MINIMAL_INTERCEPTED_1
            .clone()
            .into_new_episode(&item);
        println!(
            "EPISODE: {:#?}\nEXPECTED: {:#?}",
            ep, *EXPECTED_INTERCEPTED_1
        );
        assert_eq!(ep, *EXPECTED_INTERCEPTED_1);

        let item = channel.items().iter().nth(15).unwrap();
        let ep = EXPECTED_MINIMAL_INTERCEPTED_2
            .clone()
            .into_new_episode(&item);
        assert_eq!(ep, *EXPECTED_INTERCEPTED_2);
        Ok(())
    }

    #[test]
    fn test_new_episode_insert() -> Result<(), Error> {
        truncate_db()?;

        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let episode = channel.items().iter().nth(14).unwrap();
        let new_ep = NewEpisode::new(&episode, 42)?;
        new_ep.insert()?;
        let ep = dbqueries::get_episode_from_pk(new_ep.title(), new_ep.show_id())?;

        assert_eq!(new_ep, ep);
        assert_eq!(&new_ep, &*EXPECTED_INTERCEPTED_1);
        assert_eq!(&*EXPECTED_INTERCEPTED_1, &ep);

        let episode = channel.items().iter().nth(15).unwrap();
        let new_ep = NewEpisode::new(&episode, 42)?;
        new_ep.insert()?;
        let ep = dbqueries::get_episode_from_pk(new_ep.title(), new_ep.show_id())?;

        assert_eq!(new_ep, ep);
        assert_eq!(&new_ep, &*EXPECTED_INTERCEPTED_2);
        assert_eq!(&*EXPECTED_INTERCEPTED_2, &ep);
        Ok(())
    }

    #[test]
    fn test_new_episode_update() -> Result<(), Error> {
        truncate_db()?;
        let old = EXPECTED_INTERCEPTED_1.clone().to_episode()?;

        let updated = &*UPDATED_DURATION_INTERCEPTED_1;
        updated.update(old.rowid())?;
        let new = dbqueries::get_episode_from_pk(old.title(), old.show_id())?;

        // Assert that updating does not change the rowid and show_id
        assert_ne!(old, new);
        assert_eq!(old.rowid(), new.rowid());
        assert_eq!(old.show_id(), new.show_id());

        assert_eq!(updated, &new);
        assert_ne!(updated, &old);
        Ok(())
    }

    #[test]
    fn test_new_episode_index() -> Result<(), Error> {
        truncate_db()?;
        let expected = &*EXPECTED_INTERCEPTED_1;

        // First insert
        assert!(expected.index().is_ok());
        // Second identical, This should take the early return path
        assert!(expected.index().is_ok());
        // Get the episode
        let old = dbqueries::get_episode_from_pk(expected.title(), expected.show_id())?;
        // Assert that NewPodcast is equal to the Indexed one
        assert_eq!(*expected, old);

        let updated = &*UPDATED_DURATION_INTERCEPTED_1;

        // Update the podcast
        assert!(updated.index().is_ok());
        // Get the new Podcast
        let new = dbqueries::get_episode_from_pk(expected.title(), expected.show_id())?;
        // Assert it's diff from the old one.
        assert_ne!(new, old);
        assert_eq!(*updated, new);
        assert_eq!(new.rowid(), old.rowid());
        assert_eq!(new.show_id(), old.show_id());
        Ok(())
    }

    #[test]
    fn test_new_episode_to_episode() -> Result<(), Error> {
        let expected = &*EXPECTED_INTERCEPTED_1;

        // Assert insert() produces the same result that you would get with to_podcast()
        truncate_db()?;
        expected.insert()?;
        let old = dbqueries::get_episode_from_pk(expected.title(), expected.show_id())?;
        let ep = expected.to_episode()?;
        assert_eq!(old, ep);

        // Same as above, diff order
        truncate_db()?;
        let ep = expected.to_episode()?;
        // This should error as a unique constrain violation
        assert!(expected.insert().is_err());
        let old = dbqueries::get_episode_from_pk(expected.title(), expected.show_id())?;
        assert_eq!(old, ep);
        Ok(())
    }
}
