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

        info!("Inserting {:?}", self.title);
        diesel::insert_into(episode)
            .values(self)
            .execute(&con)
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
            .execute(&con)
            .map_err(From::from)
            .map(|_| ())
    }
}

impl Index for NewEpisode {
    // Does not update the episode description if it's the only thing that has changed.
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
            && (self.guid() == other.guid()) && (self.podcast_id() == other.podcast_id())
    }
}

impl PartialEq<Episode> for NewEpisode {
    fn eq(&self, other: &Episode) -> bool {
        (self.title() == other.title()) && (self.uri() == other.uri())
            && (self.duration() == other.duration()) && (self.epoch() == other.epoch())
            && (self.guid() == other.guid()) && (self.podcast_id() == other.podcast_id())
            && (self.description() == other.description())
            && (self.length() == other.length())
    }
}

impl NewEpisode {
    /// Parses an `rss::Item` into a `NewEpisode` Struct.
    #[allow(dead_code)]
    pub(crate) fn new(item: &rss::Item, podcast_id: i32) -> Result<Self> {
        NewEpisodeMinimal::new(item, podcast_id).map(|ep| ep.into_new_episode(item))
    }

    #[allow(dead_code)]
    pub(crate) fn to_episode(&self) -> Result<Episode> {
        self.index()?;
        dbqueries::get_episode_from_pk(&self.title, self.podcast_id)
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

    pub(crate) fn podcast_id(&self) -> i32 {
        self.podcast_id
    }
}

#[derive(Insertable, AsChangeset)]
#[table_name = "episode"]
#[derive(Debug, Clone, Builder, PartialEq)]
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
            && (self.guid() == other.guid()) && (self.podcast_id() == other.podcast_id())
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

        let duration = parser::parse_itunes_duration(item.itunes_ext());

        NewEpisodeMinimalBuilder::default()
            .title(title)
            .uri(uri)
            .duration(duration)
            .epoch(epoch)
            .guid(guid)
            .podcast_id(parent_id)
            .build()
            .map_err(From::from)
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
#[cfg(test)]
mod tests {
    use database::truncate_db;
    use dbqueries;
    use models::*;
    use models::new_episode::{NewEpisodeMinimal, NewEpisodeMinimalBuilder};

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
                .duration(Some(4171))
                .podcast_id(42)
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
                .duration(Some(4415))
                .podcast_id(42)
                .build()
                .unwrap()
        };

        static ref EXPECTED_INTERCEPTED_1: NewEpisode = {
            let descr = "NSA whistleblower Edward Snowden discusses the massive Equifax data breach \
                         and allegations of Russian interference in the US election. Commentator \
                         Shaun King explains his call for a boycott of the NFL and talks about his \
                         campaign to bring violent neo-Nazis to justice. Rapper Open Mike Eagle \
                         performs.";

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
                .podcast_id(42)
                .build()
                .unwrap()
        };

        static ref EXPECTED_INTERCEPTED_2: NewEpisode = {
            let descr = "This week on Intercepted: Jeremy gives an update on the aftermath of \
                         Blackwater’s 2007 massacre of Iraqi civilians. Intercept reporter Lee Fang \
                         lays out how a network of libertarian think tanks called the Atlas Network \
                         is insidiously shaping political infrastructure in Latin America. We speak \
                         with attorney and former Hugo Chavez adviser Eva Golinger about the \
                         Venezuela\'s political turmoil.And we hear Claudia Lizardo of the \
                         Caracas-based band, La Pequeña Revancha, talk about her music and hopes for \
                         Venezuela.";

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
                .podcast_id(42)
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
                .podcast_id(42)
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
                .epoch(1505280282)
                .duration(Some(5733))
                .podcast_id(42)
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
                .duration(Some(4491))
                .podcast_id(42)
                .build()
                .unwrap()
        };

        static ref EXPECTED_LUP_1: NewEpisode = {
            let descr = "Audit your network with a couple of easy commands on Kali Linux. Chris \
                         decides to blow off a little steam by attacking his IoT devices, Wes has the \
                         scope on Equifax blaming open source &amp; the Beard just saved the show. \
                         It’s a really packed episode!";

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
                .podcast_id(42)
                .build()
                .unwrap()
        };

        static ref EXPECTED_LUP_2: NewEpisode = {
            let descr = "The Gnome project is about to solve one of our audience's biggest Wayland’s \
                         concerns. But as the project takes on a new level of relevance, decisions for the \
                         next version of Gnome have us worried about the future.\nPlus we chat with Wimpy \
                         about the Ubuntu Rally in NYC, Microsoft’s sneaky move to turn Windows 10 into the \
                         “ULTIMATE LINUX RUNTIME”, community news &amp; more!";

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
                .podcast_id(42)
                .build()
                .unwrap()
        };
    }

    #[test]
    fn test_new_episode_minimal_intercepted() {
        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let episode = channel.items().iter().nth(14).unwrap();
        let ep = NewEpisodeMinimal::new(&episode, 42).unwrap();
        assert_eq!(ep, *EXPECTED_MINIMAL_INTERCEPTED_1);

        let episode = channel.items().iter().nth(15).unwrap();
        let ep = NewEpisodeMinimal::new(&episode, 42).unwrap();
        assert_eq!(ep, *EXPECTED_MINIMAL_INTERCEPTED_2);
    }

    #[test]
    fn test_new_episode_intercepted() {
        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let episode = channel.items().iter().nth(14).unwrap();
        let ep = NewEpisode::new(&episode, 42).unwrap();
        assert_eq!(ep, *EXPECTED_INTERCEPTED_1);

        let episode = channel.items().iter().nth(15).unwrap();
        let ep = NewEpisode::new(&episode, 42).unwrap();

        assert_eq!(ep, *EXPECTED_INTERCEPTED_2);
    }

    #[test]
    fn test_new_episode_minimal_lup() {
        let file = File::open("tests/feeds/2018-01-20-LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let episode = channel.items().iter().nth(18).unwrap();
        let ep = NewEpisodeMinimal::new(&episode, 42).unwrap();
        assert_eq!(ep, *EXPECTED_MINIMAL_LUP_1);

        let episode = channel.items().iter().nth(19).unwrap();
        let ep = NewEpisodeMinimal::new(&episode, 42).unwrap();
        assert_eq!(ep, *EXPECTED_MINIMAL_LUP_2);
    }

    #[test]
    fn test_new_episode_lup() {
        let file = File::open("tests/feeds/2018-01-20-LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let episode = channel.items().iter().nth(18).unwrap();
        let ep = NewEpisode::new(&episode, 42).unwrap();
        assert_eq!(ep, *EXPECTED_LUP_1);

        let episode = channel.items().iter().nth(19).unwrap();
        let ep = NewEpisode::new(&episode, 42).unwrap();
        assert_eq!(ep, *EXPECTED_LUP_2);
    }

    #[test]
    fn test_minimal_into_new_episode() {
        truncate_db().unwrap();

        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let item = channel.items().iter().nth(14).unwrap();
        let ep = EXPECTED_MINIMAL_INTERCEPTED_1
            .clone()
            .into_new_episode(&item);
        assert_eq!(ep, *EXPECTED_INTERCEPTED_1);

        let item = channel.items().iter().nth(15).unwrap();
        let ep = EXPECTED_MINIMAL_INTERCEPTED_2
            .clone()
            .into_new_episode(&item);
        assert_eq!(ep, *EXPECTED_INTERCEPTED_2);
    }

    #[test]
    fn test_new_episode_insert() {
        truncate_db().unwrap();

        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let episode = channel.items().iter().nth(14).unwrap();
        let new_ep = NewEpisode::new(&episode, 42).unwrap();
        new_ep.insert().unwrap();
        let ep = dbqueries::get_episode_from_pk(new_ep.title(), new_ep.podcast_id()).unwrap();

        assert_eq!(new_ep, ep);
        assert_eq!(&new_ep, &*EXPECTED_INTERCEPTED_1);
        assert_eq!(&*EXPECTED_INTERCEPTED_1, &ep);

        let episode = channel.items().iter().nth(15).unwrap();
        let new_ep = NewEpisode::new(&episode, 42).unwrap();
        new_ep.insert().unwrap();
        let ep = dbqueries::get_episode_from_pk(new_ep.title(), new_ep.podcast_id()).unwrap();

        assert_eq!(new_ep, ep);
        assert_eq!(&new_ep, &*EXPECTED_INTERCEPTED_2);
        assert_eq!(&*EXPECTED_INTERCEPTED_2, &ep);
    }

    #[test]
    fn test_new_episode_update() {
        truncate_db().unwrap();
        let old = EXPECTED_INTERCEPTED_1.clone().to_episode().unwrap();

        let updated = &*UPDATED_DURATION_INTERCEPTED_1;
        updated.update(old.rowid()).unwrap();
        let mut new = dbqueries::get_episode_from_pk(old.title(), old.podcast_id()).unwrap();

        // Assert that updating does not change the rowid and podcast_id
        assert_ne!(old, new);
        assert_eq!(old.rowid(), new.rowid());
        assert_eq!(old.podcast_id(), new.podcast_id());

        assert_eq!(updated, &new);
        assert_ne!(updated, &old);

        new.set_archive(true);
        new.save().unwrap();

        let new2 = dbqueries::get_episode_from_pk(old.title(), old.podcast_id()).unwrap();
        assert_eq!(true, new2.archive());
    }

    #[test]
    fn test_new_episode_index() {
        truncate_db().unwrap();
        let expected = &*EXPECTED_INTERCEPTED_1;

        // First insert
        assert!(expected.index().is_ok());
        // Second identical, This should take the early return path
        assert!(expected.index().is_ok());
        // Get the episode
        let old = dbqueries::get_episode_from_pk(expected.title(), expected.podcast_id()).unwrap();
        // Assert that NewPodcast is equal to the Indexed one
        assert_eq!(*expected, old);

        let updated = &*UPDATED_DURATION_INTERCEPTED_1;

        // Update the podcast
        assert!(updated.index().is_ok());
        // Get the new Podcast
        let new = dbqueries::get_episode_from_pk(expected.title(), expected.podcast_id()).unwrap();
        // Assert it's diff from the old one.
        assert_ne!(new, old);
        assert_eq!(*updated, new);
        assert_eq!(new.rowid(), old.rowid());
        assert_eq!(new.podcast_id(), old.podcast_id());
    }

    #[test]
    fn test_new_episode_to_episode() {
        let expected = &*EXPECTED_INTERCEPTED_1;
        let updated = &*UPDATED_DURATION_INTERCEPTED_1;

        // Assert insert() produces the same result that you would get with to_podcast()
        truncate_db().unwrap();
        expected.insert().unwrap();
        let old = dbqueries::get_episode_from_pk(expected.title(), expected.podcast_id()).unwrap();
        let ep = expected.to_episode().unwrap();
        assert_eq!(old, ep);

        // Same as above, diff order
        truncate_db().unwrap();
        let ep = expected.to_episode().unwrap();
        // This should error as a unique constrain violation
        assert!(expected.insert().is_err());
        let mut old =
            dbqueries::get_episode_from_pk(expected.title(), expected.podcast_id()).unwrap();
        assert_eq!(old, ep);

        old.set_archive(true);
        old.save().unwrap();

        // Assert that it does not mess with user preferences
        let ep = updated.to_episode().unwrap();
        let old = dbqueries::get_episode_from_pk(expected.title(), expected.podcast_id()).unwrap();
        assert_eq!(old, ep);
        assert_eq!(old.archive(), true);
    }
}
