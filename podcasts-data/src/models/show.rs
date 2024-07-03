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
use crate::models::{Source, SourceId};
use crate::schema::shows;

use crate::database::connection;
use crate::utils::{calculate_hash, u64_to_vec_u8, vec_u8_to_u64};
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::query_dsl::filter_dsl::FilterDsl;
use diesel::{ExpressionMethods, RunQueryDsl};

use crate::models::IdType;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;
use diesel::sqlite::Sqlite;
#[derive(AsExpression, FromSqlRow, Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
#[diesel(sql_type = diesel::sql_types::Integer)]
pub struct ShowId(pub i32);

impl<DB> FromSql<Integer, DB> for ShowId
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        i32::from_sql(bytes).map(ShowId)
    }
}

impl ToSql<diesel::sql_types::Integer, Sqlite> for ShowId {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <i32 as ToSql<Integer, Sqlite>>::to_sql(&self.0, out)
    }
}

impl IdType for ShowId {
    fn to_int(&self) -> i32 {
        self.0
    }
}

#[derive(Queryable, Identifiable, AsChangeset, Associations, PartialEq)]
#[diesel(belongs_to(Source, foreign_key = source_id))]
#[diesel(treat_none_as_null = true)]
#[diesel(table_name = shows)]
#[derive(Debug, Clone)]
/// Diesel Model of the shows table.
pub struct Show {
    id: ShowId,
    title: String,
    link: String,
    description: String,
    image_uri: Option<String>,
    image_uri_hash: Option<Vec<u8>>,
    image_cached: NaiveDateTime,
    source_id: SourceId,
}

impl Show {
    /// Get the Feed `id`.
    pub fn id(&self) -> ShowId {
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
        self.image_uri.as_deref()
    }

    /// Get the `image_uri_hash`.
    pub fn image_uri_hash(&self) -> Option<u64> {
        if let Some(b) = &self.image_uri_hash {
            return Some(vec_u8_to_u64(b.clone()));
        }
        None
    }

    /// Get the `image_cached`.
    pub fn image_cached(&self) -> &NaiveDateTime {
        &self.image_cached
    }

    /// `Source` table foreign key.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }
}

#[derive(Queryable, Debug, Clone)]
/// Diesel Model of the Show cover query.
/// Used for fetching information about a Show's cover.
pub struct ShowCoverModel {
    id: ShowId,
    title: String,
    image_uri: Option<String>,
    image_uri_hash: Option<Vec<u8>>,
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
    pub fn id(&self) -> ShowId {
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
        self.image_uri.as_deref()
    }

    /// Get the `image_uri_hash`.
    pub fn image_uri_hash(&self) -> Option<u64> {
        if let Some(b) = &self.image_uri_hash {
            return Some(vec_u8_to_u64(b.clone()));
        }
        None
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

    /// Update the timestamp when the image has been cached.
    pub(crate) fn update_image_cached(&self) -> Result<(), DataError> {
        use crate::schema::shows::dsl::*;
        let db = connection();
        let mut con = db.get()?;
        diesel::update(shows.filter(id.eq(self.id)))
            .set(image_cached.eq(Utc::now().naive_utc()))
            .execute(&mut con)
            .map(|_| ())
            .map_err(From::from)
    }

    /// Update the hash of the image's URI.
    fn update_image_uri_hash(&self) -> Result<(), DataError> {
        use crate::schema::shows::dsl::*;
        let db = connection();
        let mut con = db.get()?;

        let mut hash: Option<Vec<u8>> = None;
        if let Some(i) = &self.image_uri {
            hash = Some(u64_to_vec_u8(calculate_hash(i)));
        }

        diesel::update(shows.filter(id.eq(self.id)))
            .set(image_uri_hash.eq(&hash))
            .execute(&mut con)
            .map(|_| ())
            .map_err(From::from)
    }

    /// Update the image's timestamp and URI hash value.
    pub fn update_image_cache_values(&self) -> Result<(), DataError> {
        match self.image_uri_hash() {
            None => self.update_image_uri_hash()?,
            Some(hash) => match self.image_uri() {
                None => self.update_image_uri_hash()?,
                Some(image_uri) => {
                    if calculate_hash(&image_uri) != hash {
                        self.update_image_uri_hash()?;
                    }
                }
            },
        }
        self.update_image_cached()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::truncate_db;
    use crate::dbqueries;
    use crate::models::{Insert, NewShow, NewShowBuilder, Update};
    use anyhow::Result;
    use once_cell::sync::Lazy;
    use std::{thread, time};

    static EXPECTED_INTERCEPTED: Lazy<NewShow> = Lazy::new(|| {
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

        NewShowBuilder::default()
            .title("Intercepted with Jeremy Scahill")
            .link("https://theintercept.com/podcasts")
            .description(descr)
            .image_uri(String::from(image_uri))
            .image_uri_hash(Some(vec![164, 62, 7, 221, 215, 202, 38, 41]))
            .image_cached(Utc::now().naive_utc())
            .source_id(42)
            .build()
            .unwrap()
    });
    static UPDATED_IMAGE_URI_INTERCEPTED: Lazy<NewShow> = Lazy::new(|| {
        let image_uri = "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3";

        NewShowBuilder::default()
            .title("Intercepted with Jeremy Scahill")
            .link("https://theintercept.com/podcasts")
            .description(EXPECTED_INTERCEPTED.description())
            .image_uri(String::from(image_uri))
            .image_uri_hash(Some(vec![164, 62, 7, 221, 215, 202, 38, 41]))
            .image_cached(EXPECTED_INTERCEPTED.image_cached().unwrap())
            .source_id(42)
            .build()
            .unwrap()
    });

    #[test]
    fn should_update_timestamp_when_update_image_cached_is_called_after_the_timestamp_has_expired(
    ) -> Result<()> {
        truncate_db()?;
        EXPECTED_INTERCEPTED.insert()?;
        let show = EXPECTED_INTERCEPTED.to_podcast()?;
        let show: ShowCoverModel = show.into();
        let original_timestamp = show.image_cached();
        show.update_image_cached().unwrap();
        let show = dbqueries::get_podcast_from_id(show.id())?;
        let updated_timestamp = show.image_cached();
        assert!(original_timestamp < updated_timestamp);

        // The image's URI and its hash should remain unchanged.
        assert_eq!(
            show.image_uri().unwrap(),
            "http://static.megaphone.fm/podcasts/d5735a50-d904-11e6-8532-73c7de466ea6/image/\
                     uploads_2F1484252190700-qhn5krasklbce3dh-a797539282700ea0298a3a26f7e49b0b_\
                     2FIntercepted_COVER%2B_281_29.png"
        );
        assert_eq!(show.image_uri_hash().unwrap(), 2965280433145069220);
        Ok(())
    }

    #[test]
    fn should_update_hash_when_update_image_uri_hash_is_called_when_the_hash_is_invalid(
    ) -> Result<()> {
        truncate_db()?;
        EXPECTED_INTERCEPTED.insert()?;
        let original = EXPECTED_INTERCEPTED.to_podcast()?;
        let original_hash: u64 = 2965280433145069220;
        let updated = &*UPDATED_IMAGE_URI_INTERCEPTED;
        updated.update(original.id())?;
        let show = dbqueries::get_podcast_cover_from_id(original.id())?;

        let not_yet_updated_hash = updated.image_uri_hash().unwrap();
        assert_eq!(not_yet_updated_hash, original_hash);

        show.update_image_uri_hash().unwrap();
        let show = dbqueries::get_podcast_from_id(original.id())?;
        let updated_hash = show.image_uri_hash().unwrap();
        let expected_updated_hash: u64 = 1748982167920802687;
        assert_eq!(updated_hash, expected_updated_hash);
        assert_eq!(
            show.image_uri().unwrap(),
            "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3"
        );
        Ok(())
    }

    #[test]
    fn should_update_timestamp_only_when_update_image_cached_values_is_called_after_the_timestamp_has_expired(
    ) -> Result<()> {
        truncate_db()?;
        EXPECTED_INTERCEPTED.insert()?;
        let show = EXPECTED_INTERCEPTED.to_podcast()?;
        let show: ShowCoverModel = show.into();
        let original_timestamp = show.image_cached();
        show.update_image_cache_values().unwrap();
        let show = dbqueries::get_podcast_from_id(show.id())?;
        let updated_timestamp = show.image_cached();
        assert!(original_timestamp < updated_timestamp);
        assert_eq!(
            show.image_uri().unwrap(),
            "http://static.megaphone.fm/podcasts/d5735a50-d904-11e6-8532-73c7de466ea6/image/\
                     uploads_2F1484252190700-qhn5krasklbce3dh-a797539282700ea0298a3a26f7e49b0b_\
                     2FIntercepted_COVER%2B_281_29.png"
        );
        assert_eq!(show.image_uri_hash().unwrap(), 2965280433145069220);
        Ok(())
    }

    #[test]
    fn should_update_timestamp_and_hash_when_update_image_cached_values_is_called_when_hash_is_invalid(
    ) -> Result<()> {
        truncate_db()?;
        EXPECTED_INTERCEPTED.insert()?;
        let original = EXPECTED_INTERCEPTED.to_podcast()?;
        let original_timestamp = original.image_cached();
        let updated = &*UPDATED_IMAGE_URI_INTERCEPTED;
        updated.update(original.id())?;
        let show = dbqueries::get_podcast_cover_from_id(original.id())?;

        let not_yet_updated_hash = show.image_uri_hash().unwrap();
        let original_hash: u64 = 2965280433145069220;
        assert_eq!(not_yet_updated_hash, original_hash);

        show.update_image_cache_values().unwrap();
        let show = dbqueries::get_podcast_from_id(show.id())?;
        let updated_timestamp = show.image_cached();
        assert!(original_timestamp < updated_timestamp);

        let updated_hash = show.image_uri_hash().unwrap();
        let expected_updated_hash: u64 = 1748982167920802687;
        assert_eq!(updated_hash, expected_updated_hash);

        assert_eq!(
            show.image_uri().unwrap(),
            "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3"
        );
        Ok(())
    }

    #[test]
    fn cached_image_should_be_valid_when_uri_and_hash_are_unchanged() -> Result<()> {
        let image_uri = String::from(
            "http://www.jupiterbroadcasting.com/wp-content/uploads/2018/01/lup-0232-v.jpg",
        );
        let hash = vec![191, 166, 24, 137, 178, 75, 5, 227];
        let cover = ShowCoverModel {
            id: 0,
            title: String::from("Linux Unplugged"),
            image_uri: Some(image_uri),
            image_uri_hash: Some(hash),
            image_cached: Utc::now().naive_utc(),
        };
        let valid = Duration::weeks(4);
        assert!(cover.is_cached_image_valid(&valid));
        Ok(())
    }

    #[test]
    fn a_different_uri_should_invalidate_cached_image() -> Result<()> {
        // The old image URI used for the hash here is:
        // http://www.jupiterbroadcasting.com/wp-content/uploads/2018/01/lup-0232-v.jpg
        let new_image_uri = String::from(
            "https://assets.fireside.fm/file/fireside-images/podcasts/images/f/f31a453c-fa15-491f-8618-3f71f1d565e5/cover.jpg?v=3",
        );
        let hash = vec![191, 166, 24, 137, 178, 75, 5, 227];
        let cover = ShowCoverModel {
            id: 0,
            title: String::from("Linux Unplugged"),
            image_uri: Some(new_image_uri),
            image_uri_hash: Some(hash),
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
        let hash = vec![191, 166, 24, 137, 178, 75, 5, 227];
        let cover = ShowCoverModel {
            id: 0,
            title: String::from("Linux Unplugged"),
            image_uri: Some(image_uri),
            image_uri_hash: Some(hash),
            image_cached: Utc::now().naive_utc(),
        };
        let valid = Duration::nanoseconds(1);
        thread::sleep(time::Duration::from_nanos(2));
        assert!(!cover.is_cached_image_valid(&valid));
        Ok(())
    }
}
