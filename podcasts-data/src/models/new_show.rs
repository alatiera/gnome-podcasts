// new_show.rs
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
use rss;

use crate::errors::DataError;
use crate::models::Show;
use crate::models::{Index, Insert, Update};
use crate::schema::shows;

use crate::database::connection;
use crate::dbqueries;
use crate::utils::url_cleaner;

#[derive(Insertable, AsChangeset)]
#[table_name = "shows"]
#[derive(Debug, Clone, Default, Builder, PartialEq)]
#[builder(default)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub(crate) struct NewShow {
    title: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    source_id: i32,
}

impl Insert<()> for NewShow {
    type Error = DataError;

    fn insert(&self) -> Result<(), Self::Error> {
        use crate::schema::shows::dsl::*;
        let db = connection();
        let con = db.get()?;

        diesel::insert_into(shows)
            .values(self)
            .execute(&con)
            .map(|_| ())
            .map_err(From::from)
    }
}

impl Update<()> for NewShow {
    type Error = DataError;

    fn update(&self, show_id: i32) -> Result<(), Self::Error> {
        use crate::schema::shows::dsl::*;
        let db = connection();
        let con = db.get()?;

        info!("Updating {}", self.title);
        diesel::update(shows.filter(id.eq(show_id)))
            .set(self)
            .execute(&con)
            .map(|_| ())
            .map_err(From::from)
    }
}

// TODO: Maybe return an Enum<Action(Resut)> Instead.
// It would make unti testing better too.
impl Index<()> for NewShow {
    type Error = DataError;

    fn index(&self) -> Result<(), DataError> {
        let exists = dbqueries::podcast_exists(self.source_id)?;

        if exists {
            let other = dbqueries::get_podcast_from_source_id(self.source_id)?;

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

impl PartialEq<Show> for NewShow {
    fn eq(&self, other: &Show) -> bool {
        (self.link() == other.link())
            && (self.title() == other.title())
            && (self.image_uri() == other.image_uri())
            && (self.description() == other.description())
            && (self.source_id() == other.source_id())
    }
}

impl NewShow {
    /// Parses a `rss::Channel` into a `NewShow` Struct.
    pub(crate) fn new(chan: &rss::Channel, source_id: i32) -> NewShow {
        let title = chan.title().trim();
        let link = url_cleaner(chan.link().trim());

        let description = ammonia::Builder::new()
            // Remove `rel` attributes from `<a>` tags
            .link_rel(None)
            .clean(chan.description().trim())
            .to_string();

        // Try to get the itunes img first
        let itunes_img = chan
            .itunes_ext()
            .and_then(|s| s.image().map(|url| url.trim()))
            .map(|s| s.to_owned());
        // If itunes is None, try to get the channel.image from the rss spec
        let image_uri = itunes_img.or_else(|| chan.image().map(|s| s.url().trim().to_owned()));

        NewShowBuilder::default()
            .title(title)
            .description(description)
            .link(link)
            .image_uri(image_uri)
            .source_id(source_id)
            .build()
            .unwrap()
    }

    // Look out for when tryinto lands into stable.
    pub(crate) fn to_podcast(&self) -> Result<Show, DataError> {
        self.index()?;
        dbqueries::get_podcast_from_source_id(self.source_id).map_err(From::from)
    }
}

// Ignore the following geters. They are used in unit tests mainly.
impl NewShow {
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;
    use rss::Channel;

    use crate::database::truncate_db;
    use crate::models::NewShowBuilder;

    use std::fs::File;
    use std::io::BufReader;

    // Pre-built expected NewShow structs.
    lazy_static! {
        static ref EXPECTED_INTERCEPTED: NewShow = {
            let descr = "The people behind The Intercept’s fearless reporting and incisive \
                         commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and \
                         others—discuss the crucial issues of our time: national security, civil \
                         liberties, foreign policy, and criminal justice.  Plus interviews with \
                         artists, thinkers, and newsmakers who challenge our preconceptions about \
                         the world we live in.";

            NewShowBuilder::default()
                .title("Intercepted with Jeremy Scahill")
                .link("https://theintercept.com/podcasts")
                .description(descr)
                .image_uri(Some(String::from(
                    "http://static.megaphone.fm/podcasts/d5735a50-d904-11e6-8532-73c7de466ea6/image/\
                     uploads_2F1484252190700-qhn5krasklbce3dh-a797539282700ea0298a3a26f7e49b0b_\
                     2FIntercepted_COVER%2B_281_29.png")
                ))
                .source_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_LUP: NewShow = {
            let descr = "An open show powered by community LINUX Unplugged takes the best \
                         attributes of open collaboration and focuses them into a weekly \
                         lifestyle show about Linux.";

            NewShowBuilder::default()
                .title("LINUX Unplugged Podcast")
                .link("http://www.jupiterbroadcasting.com/")
                .description(descr)
                .image_uri(Some(String::from(
                    "http://www.jupiterbroadcasting.com/images/LASUN-Badge1400.jpg",
                )))
                .source_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_TIPOFF: NewShow = {
            let desc = "<p>Welcome to The Tip Off- the podcast where we take you behind the \
                        scenes of some of the best investigative journalism from recent years. \
                        Each episode we’ll be digging into an investigative scoop- hearing from \
                        the journalists behind the work as they tell us about the leads, the \
                        dead-ends and of course, the tip offs. There’ll be car chases, slammed \
                        doors, terrorist cells, meetings in dimly lit bars and cafes, wrangling \
                        with despotic regimes and much more. So if you’re curious about the fun, \
                        complicated detective work that goes into doing great investigative \
                        journalism- then this is the podcast for you.</p>";

            NewShowBuilder::default()
                .title("The Tip Off")
                .link("http://www.acast.com/thetipoff")
                .description(desc)
                .image_uri(Some(String::from(
                    "https://imagecdn.acast.com/image?h=1500&w=1500&source=http%3A%2F%2Fi1.sndcdn.\
                     com%2Favatars-000317856075-a2coqz-original.jpg",
                )))
                .source_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_STARS: NewShow = {
            let descr = "<p>The first audio drama from Tor Labs and Gideon Media, Steal the Stars \
                         is a gripping noir science fiction thriller in 14 episodes: Forbidden \
                         love, a crashed UFO, an alien body, and an impossible heist unlike any \
                         ever attempted - scripted by Mac Rogers, the award-winning playwright \
                         and writer of the multi-million download The Message and LifeAfter.</p>";
            let img = "https://dfkfj8j276wwv.cloudfront.net/images/2c/5f/a0/1a/2c5fa01a-ae78-4a8c-\
                       b183-7311d2e436c3/b3a4aa57a576bb662191f2a6bc2a436c8c4ae256ecffaff5c4c54fd42e\
                       923914941c264d01efb1833234b52c9530e67d28a8cebbe3d11a4bc0fbbdf13ecdf1c3.jpeg";

            NewShowBuilder::default()
                .title("Steal the Stars")
                .link("http://tor-labs.com/")
                .description(descr)
                .image_uri(Some(String::from(img)))
                .source_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_CODE: NewShow = {
            let descr = "A podcast about humans and technology. Panelists: Coraline Ada Ehmke, \
                         David Brady, Jessica Kerr, Jay Bobo, Astrid Countee and Sam \
                         Livingston-Gray. Brought to you by @therubyrep.";

            NewShowBuilder::default()
                .title("Greater Than Code")
                .link("https://www.greaterthancode.com/")
                .description(descr)
                .image_uri(Some(String::from(
                    "http://www.greaterthancode.com/wp-content/uploads/2016/10/code1400-4.jpg",
                )))
                .source_id(42)
                .build()
                .unwrap()
        };
        static ref EXPECTED_ELLINOFRENEIA: NewShow = {
            NewShowBuilder::default()
                .title("Ελληνοφρένεια")
                .link("https://ellinofreneia.sealabs.net/feed.rss")
                .description("Ανεπίσημο feed της Ελληνοφρένειας")
                .image_uri(Some("https://ellinofreneia.sealabs.net/logo.png".into()))
                .source_id(42)
                .build()
                .unwrap()
        };
        static ref UPDATED_DESC_INTERCEPTED: NewShow = {
            NewShowBuilder::default()
                .title("Intercepted with Jeremy Scahill")
                .link("https://theintercept.com/podcasts")
                .description("New Description")
                .image_uri(Some(String::from(
                    "http://static.megaphone.fm/podcasts/d5735a50-d904-11e6-8532-73c7de466ea6/image/\
                     uploads_2F1484252190700-qhn5krasklbce3dh-a797539282700ea0298a3a26f7e49b0b_\
                     2FIntercepted_COVER%2B_281_29.png")
                ))
                .source_id(42)
                .build()
                .unwrap()
        };
    }

    #[test]
    fn test_new_podcast_intercepted() -> Result<()> {
        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let pd = NewShow::new(&channel, 42);
        assert_eq!(*EXPECTED_INTERCEPTED, pd);
        Ok(())
    }

    #[test]
    fn test_new_podcast_lup() -> Result<()> {
        let file = File::open("tests/feeds/2018-01-20-LinuxUnplugged.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let pd = NewShow::new(&channel, 42);
        assert_eq!(*EXPECTED_LUP, pd);
        Ok(())
    }

    #[test]
    fn test_new_podcast_thetipoff() -> Result<()> {
        let file = File::open("tests/feeds/2018-01-20-TheTipOff.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let pd = NewShow::new(&channel, 42);
        assert_eq!(*EXPECTED_TIPOFF, pd);
        Ok(())
    }

    #[test]
    fn test_new_podcast_steal_the_stars() -> Result<()> {
        let file = File::open("tests/feeds/2018-01-20-StealTheStars.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let pd = NewShow::new(&channel, 42);
        assert_eq!(*EXPECTED_STARS, pd);
        Ok(())
    }

    #[test]
    fn test_new_podcast_greater_than_code() -> Result<()> {
        let file = File::open("tests/feeds/2018-01-20-GreaterThanCode.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let pd = NewShow::new(&channel, 42);
        assert_eq!(*EXPECTED_CODE, pd);
        Ok(())
    }

    #[test]
    fn test_new_podcast_ellinofreneia() -> Result<()> {
        let file = File::open("tests/feeds/2018-03-28-Ellinofreneia.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let pd = NewShow::new(&channel, 42);
        assert_eq!(*EXPECTED_ELLINOFRENEIA, pd);
        Ok(())
    }

    #[test]
    // This maybe could be a doc test on insert.
    fn test_new_podcast_insert() -> Result<()> {
        truncate_db()?;
        let file = File::open("tests/feeds/2018-01-20-Intercepted.xml")?;
        let channel = Channel::read_from(BufReader::new(file))?;

        let npd = NewShow::new(&channel, 42);
        npd.insert()?;
        let pd = dbqueries::get_podcast_from_source_id(42)?;

        assert_eq!(npd, pd);
        assert_eq!(*EXPECTED_INTERCEPTED, npd);
        assert_eq!(&*EXPECTED_INTERCEPTED, &pd);
        Ok(())
    }

    #[test]
    // TODO: Add more test/checks
    // Currently there's a test that only checks new description or title.
    // If you have time and want to help, implement the test for the other fields
    // too.
    fn test_new_podcast_update() -> Result<()> {
        truncate_db()?;
        let old = EXPECTED_INTERCEPTED.to_podcast()?;

        let updated = &*UPDATED_DESC_INTERCEPTED;
        updated.update(old.id())?;
        let new = dbqueries::get_podcast_from_source_id(42)?;

        assert_ne!(old, new);
        assert_eq!(old.id(), new.id());
        assert_eq!(old.source_id(), new.source_id());
        assert_eq!(updated, &new);
        assert_ne!(updated, &old);
        Ok(())
    }

    #[test]
    fn test_new_podcast_index() -> Result<()> {
        truncate_db()?;

        // First insert
        assert!(EXPECTED_INTERCEPTED.index().is_ok());
        // Second identical, This should take the early return path
        assert!(EXPECTED_INTERCEPTED.index().is_ok());
        // Get the podcast
        let old = dbqueries::get_podcast_from_source_id(42)?;
        // Assert that NewShow is equal to the Indexed one
        assert_eq!(&*EXPECTED_INTERCEPTED, &old);

        let updated = &*UPDATED_DESC_INTERCEPTED;

        // Update the podcast
        assert!(updated.index().is_ok());
        // Get the new Show
        let new = dbqueries::get_podcast_from_source_id(42)?;
        // Assert it's diff from the old one.
        assert_ne!(new, old);
        assert_eq!(new.id(), old.id());
        assert_eq!(new.source_id(), old.source_id());
        Ok(())
    }

    #[test]
    fn test_to_podcast() -> Result<()> {
        // Assert insert() produces the same result that you would get with to_podcast()
        truncate_db()?;
        EXPECTED_INTERCEPTED.insert()?;
        let old = dbqueries::get_podcast_from_source_id(42)?;
        let pd = EXPECTED_INTERCEPTED.to_podcast()?;
        assert_eq!(old, pd);

        // Same as above, diff order
        truncate_db()?;
        let pd = EXPECTED_INTERCEPTED.to_podcast()?;
        // This should error as a unique constrain violation
        assert!(EXPECTED_INTERCEPTED.insert().is_err());
        let old = dbqueries::get_podcast_from_source_id(42)?;
        assert_eq!(old, pd);
        Ok(())
    }
}
