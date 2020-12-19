// show.rs
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

use crate::errors::DataError;
use crate::models::Source;
use crate::schema::shows;

use crate::database::connection;
use crate::utils::calculate_hash;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::query_dsl::filter_dsl::FilterDsl;
use diesel::{ExpressionMethods, RunQueryDsl};

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
    image_uri_hash: Option<i64>,
    image_cached: NaiveDateTime,
    source_id: i32,
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

    /// Get the `description`.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the `image_uri`.
    ///
    /// Represents the uri(url usually) that the Feed cover image is located at.
    pub fn image_uri(&self) -> Option<&str> {
        self.image_uri.as_ref().map(|s| s.as_str())
    }

    /// Get the `image_uri_hash`.
    pub fn image_uri_hash(&self) -> Option<i64> {
        self.image_uri_hash
    }

    /// Get the `image_cached`.
    pub fn image_cached(&self) -> &NaiveDateTime {
        &self.image_cached
    }

    /// `Source` table foreign key.
    pub fn source_id(&self) -> i32 {
        self.source_id
    }

    /// Update the hash of the image's URI.
    pub fn update_image_uri_hash(&self) -> Result<(), DataError> {
        use crate::schema::shows::dsl::*;
        let db = connection();
        let con = db.get()?;
        diesel::update(shows.filter(id.eq(self.id)))
            .set(image_uri_hash.eq(Some(calculate_hash(&self.image_uri))))
            .execute(&con)
            .map(|_| ())
            .map_err(From::from)
    }

    /// Update the timestamp when the image has been cached.
    pub fn update_image_cached(&self) -> Result<(), DataError> {
        use crate::schema::shows::dsl::*;
        let db = connection();
        let con = db.get()?;
        diesel::update(shows.filter(id.eq(self.id)))
            .set(image_cached.eq(Utc::now().naive_utc()))
            .execute(&con)
            .map(|_| ())
            .map_err(From::from)
    }

    /// Update the image's timestamp and URI hash value.
    pub fn update_image_cache_values(&self) -> Result<(), DataError> {
        match self.image_uri_hash() {
            None => self.update_image_uri_hash()?,
            Some(hash) => {
                if calculate_hash(&self.image_uri()) != hash {
                    self.update_image_uri_hash()?;
                }
            }
        }
        match self.update_image_cached() {
            Ok(s) => Ok(s),
            Err(e) => Err(e),
        }
    }
}

#[derive(Queryable, Debug, Clone)]
/// Diesel Model of the Show cover query.
/// Used for fetching information about a Show's cover.
pub struct ShowCoverModel {
    id: i32,
    title: String,
    image_uri: Option<String>,
    image_uri_hash: Option<i64>,
    image_cached: NaiveDateTime,
}

impl From<Show> for ShowCoverModel {
    fn from(p: Show) -> ShowCoverModel {
        ShowCoverModel {
            id: p.id,
            title: p.title,
            image_uri: p.image_uri,
            image_uri_hash: p.image_uri_hash,
            image_cached: p.image_cached,
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

    /// Get the `image_uri_hash`.
    pub fn image_uri_hash(&self) -> Option<i64> {
        self.image_uri_hash
    }

    /// Get the `image_cached`.
    pub fn image_cached(&self) -> &NaiveDateTime {
        &self.image_cached
    }

    /// Determine whether a cached image is valid.
    ///
    /// A cached image is valid from the time of its previous download for the given length of time.
    /// Otherwise, a cached image is invalidated when the hash of its URI has changed.
    pub fn is_cached_image_valid(&self, valid: &Duration) -> bool {
        if Utc::now()
            .naive_utc()
            .signed_duration_since(*self.image_cached())
            > *valid
        {
            return false;
        }
        if let Some(new) = &self.image_uri() {
            if let Some(orig) = self.image_uri_hash() {
                return calculate_hash(new) == orig;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::truncate_db;
    use crate::dbqueries;
    use crate::models::{Insert, NewShow, NewShowBuilder, Update};
    use anyhow::Result;
    use std::{thread, time};

    lazy_static! {
        static ref EXPECTED_INTERCEPTED: NewShow = {
            let descr = "The people behind The Intercept’s fearless reporting and incisive \
                         commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and \
                         others—discuss the crucial issues of our time: national security, civil \
                         liberties, foreign policy, and criminal justice.  Plus interviews with \
                         artists, thinkers, and newsmakers who challenge our preconceptions about \
                         the world we live in.";

            let image_uri =
                "http://static.megaphone.fm/podcasts/d5735a50-d904-11e6-8532-73c7de466ea6/image/\
                 uploads_2F1484252190700-qhn5krasklbce3dh-a797539282700ea0298a3a26f7e49b0b_\
                 2FIntercepted_COVER%2B_281_29.png";
            let image_uri_hash = calculate_hash(&image_uri);

            NewShowBuilder::default()
                .title("Intercepted with Jeremy Scahill")
                .link("https://theintercept.com/podcasts")
                .description(descr)
                .image_uri(String::from(image_uri))
                .image_uri_hash(image_uri_hash)
                .image_cached(Utc::now().naive_utc())
                .source_id(42)
                .build()
                .unwrap()
        };
        static ref UPDATED_IMAGE_URI_INTERCEPTED: NewShow = {
            let image_uri = "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3";

            NewShowBuilder::default()
                .title("Intercepted with Jeremy Scahill")
                .link("https://theintercept.com/podcasts")
                .description(EXPECTED_INTERCEPTED.description())
                .image_uri(String::from(image_uri))
                .image_uri_hash(EXPECTED_INTERCEPTED.image_uri_hash().unwrap())
                .image_cached(EXPECTED_INTERCEPTED.image_cached().unwrap())
                .source_id(42)
                .build()
                .unwrap()
        };
    }

    #[test]
    fn update_image_cached_timestamp() -> Result<()> {
        truncate_db()?;
        EXPECTED_INTERCEPTED.insert()?;
        let show = EXPECTED_INTERCEPTED.to_podcast()?;
        let original_timestamp = show.image_cached();
        show.update_image_cached().unwrap();
        let show = dbqueries::get_podcast_from_id(show.id())?;
        let updated_timestamp = show.image_cached();
        assert!(original_timestamp < updated_timestamp);
        assert_eq!(show.title(), "Intercepted with Jeremy Scahill");
        assert_eq!(show.link(), "https://theintercept.com/podcasts");
        assert_eq!(
            show.description(),
            "The people behind The Intercept’s fearless reporting and incisive \
                         commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and \
                         others—discuss the crucial issues of our time: national security, civil \
                         liberties, foreign policy, and criminal justice.  Plus interviews with \
                         artists, thinkers, and newsmakers who challenge our preconceptions about \
                         the world we live in."
        );
        assert_eq!(
            show.image_uri().unwrap(),
            "http://static.megaphone.fm/podcasts/d5735a50-d904-11e6-8532-73c7de466ea6/image/\
                     uploads_2F1484252190700-qhn5krasklbce3dh-a797539282700ea0298a3a26f7e49b0b_\
                     2FIntercepted_COVER%2B_281_29.png"
        );
        assert_eq!(show.image_uri_hash(), EXPECTED_INTERCEPTED.image_uri_hash());
        assert_eq!(show.source_id(), 42);
        Ok(())
    }

    #[test]
    fn update_image_uri_hash() -> Result<()> {
        truncate_db()?;
        EXPECTED_INTERCEPTED.insert()?;
        let original = EXPECTED_INTERCEPTED.to_podcast()?;
        let original_hash = original.image_uri_hash();
        let updated = &*UPDATED_IMAGE_URI_INTERCEPTED;
        updated.update(original.id())?;
        let show = dbqueries::get_podcast_from_id(original.id())?;
        let not_updated_hash = updated.image_uri_hash();
        assert_eq!(original_hash, not_updated_hash);
        show.update_image_uri_hash().unwrap();
        let show = dbqueries::get_podcast_from_id(original.id())?;
        let updated_hash = show.image_uri_hash();
        assert_ne!(original_hash, updated_hash);
        assert_eq!(show.title(), "Intercepted with Jeremy Scahill");
        assert_eq!(show.link(), "https://theintercept.com/podcasts");
        assert_eq!(
            show.description(),
            "The people behind The Intercept’s fearless reporting and incisive \
                         commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and \
                         others—discuss the crucial issues of our time: national security, civil \
                         liberties, foreign policy, and criminal justice.  Plus interviews with \
                         artists, thinkers, and newsmakers who challenge our preconceptions about \
                         the world we live in."
        );
        assert_eq!(
            show.image_uri().unwrap(),
            "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3"
        );
        assert_eq!(show.source_id(), 42);
        Ok(())
    }

    #[test]
    fn update_image_cached_values_timestamp_only() -> Result<()> {
        truncate_db()?;
        EXPECTED_INTERCEPTED.insert()?;
        let show = EXPECTED_INTERCEPTED.to_podcast()?;
        let original_timestamp = show.image_cached();
        show.update_image_cache_values().unwrap();
        let show = dbqueries::get_podcast_from_id(show.id())?;
        let updated_timestamp = show.image_cached();
        assert!(original_timestamp < updated_timestamp);
        assert_ne!(show.image_uri_hash(), EXPECTED_INTERCEPTED.image_uri_hash());
        assert_eq!(show.title(), "Intercepted with Jeremy Scahill");
        assert_eq!(show.link(), "https://theintercept.com/podcasts");
        assert_eq!(
            show.description(),
            "The people behind The Intercept’s fearless reporting and incisive \
                         commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and \
                         others—discuss the crucial issues of our time: national security, civil \
                         liberties, foreign policy, and criminal justice.  Plus interviews with \
                         artists, thinkers, and newsmakers who challenge our preconceptions about \
                         the world we live in."
        );
        assert_eq!(
            show.image_uri().unwrap(),
            "http://static.megaphone.fm/podcasts/d5735a50-d904-11e6-8532-73c7de466ea6/image/\
                     uploads_2F1484252190700-qhn5krasklbce3dh-a797539282700ea0298a3a26f7e49b0b_\
                     2FIntercepted_COVER%2B_281_29.png"
        );
        assert_eq!(show.source_id(), 42);
        Ok(())
    }

    #[test]
    fn update_image_cached_values_timestamp_and_hash() -> Result<()> {
        truncate_db()?;
        EXPECTED_INTERCEPTED.insert()?;
        let original = EXPECTED_INTERCEPTED.to_podcast()?;
        let original_timestamp = original.image_cached();
        let original_hash = original.image_uri_hash();
        let updated = &*UPDATED_IMAGE_URI_INTERCEPTED;
        updated.update(original.id())?;
        let show = dbqueries::get_podcast_from_id(original.id())?;
        let not_updated_hash = show.image_uri_hash();
        assert_eq!(original_hash, not_updated_hash);
        show.update_image_cache_values().unwrap();
        let show = dbqueries::get_podcast_from_id(show.id())?;
        let updated_timestamp = show.image_cached();
        assert!(original_timestamp < updated_timestamp);
        assert_ne!(show.image_uri_hash(), original_hash);
        assert_eq!(show.title(), "Intercepted with Jeremy Scahill");
        assert_eq!(show.link(), "https://theintercept.com/podcasts");
        assert_eq!(
            show.description(),
            "The people behind The Intercept’s fearless reporting and incisive \
                         commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and \
                         others—discuss the crucial issues of our time: national security, civil \
                         liberties, foreign policy, and criminal justice.  Plus interviews with \
                         artists, thinkers, and newsmakers who challenge our preconceptions about \
                         the world we live in."
        );
        assert_eq!(
            show.image_uri().unwrap(),
            "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3"
        );
        assert_eq!(show.source_id(), 42);
        Ok(())
    }

    #[test]
    fn cached_image_should_be_valid_when_uri_and_hash_are_unchanged() -> Result<()> {
        let image_uri = String::from(
            "http://www.jupiterbroadcasting.com/wp-content/uploads/2018/01/lup-0232-v.jpg",
        );
        let image_uri_hash = calculate_hash(&image_uri);
        let cover = ShowCoverModel {
            id: 0,
            title: String::from("Linux Unplugged"),
            image_uri: Some(image_uri),
            image_uri_hash: Some(image_uri_hash),
            image_cached: Utc::now().naive_utc(),
        };
        let valid = Duration::weeks(4);
        assert!(cover.is_cached_image_valid(&valid));
        Ok(())
    }

    #[test]
    fn a_different_uri_should_invalidate_cached_image() -> Result<()> {
        let old_image_uri = String::from(
            "http://www.jupiterbroadcasting.com/wp-content/uploads/2018/01/lup-0232-v.jpg",
        );
        let old_image_uri_hash = calculate_hash(&old_image_uri);
        let new_image_uri = String::from(
            "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3",
        );
        let cover = ShowCoverModel {
            id: 0,
            title: String::from("Linux Unplugged"),
            image_uri: Some(new_image_uri),
            image_uri_hash: Some(old_image_uri_hash),
            image_cached: Utc::now().naive_utc(),
        };
        let valid = Duration::weeks(4);
        assert!(!cover.is_cached_image_valid(&valid));
        Ok(())
    }

    #[test]
    fn cached_image_should_be_invalidated_after_valid_duration() -> Result<()> {
        let image_uri = String::from(
            "http://www.jupiterbroadcasting.com/wp-content/uploads/2018/01/lup-0232-v.jpg",
        );
        let image_uri_hash = calculate_hash(&image_uri);
        let cover = ShowCoverModel {
            id: 0,
            title: String::from("Linux Unplugged"),
            image_uri: Some(image_uri),
            image_uri_hash: Some(image_uri_hash),
            image_cached: Utc::now().naive_utc(),
        };
        let valid = Duration::nanoseconds(1);
        thread::sleep(time::Duration::from_nanos(2));
        assert!(!cover.is_cached_image_valid(&valid));
        Ok(())
    }
}
