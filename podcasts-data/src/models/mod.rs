// mod.rs
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

mod new_episode;
mod new_show;
mod new_source;

mod discovery_settings;
mod episode;
mod show;
mod source;
/// Sync datatypes to store updates that still have to be sent out.
/// This is mostly glue code for the DB, use store(), fetch(), delete() methods to interact.
pub mod sync;

pub(crate) use self::discovery_settings::DiscoverySetting;

pub(crate) use self::new_episode::{NewEpisode, NewEpisodeMinimal};
pub(crate) use self::new_show::NewShow;
pub(crate) use self::new_source::NewSource;

#[cfg(test)]
pub(crate) use self::new_episode::NewEpisodeBuilder;
#[cfg(test)]
pub(crate) use self::new_show::NewShowBuilder;

pub use self::episode::{
    Episode, EpisodeCleanerModel, EpisodeId, EpisodeMinimal, EpisodeModel, EpisodeWidgetModel,
};
pub use self::show::{Show, ShowCoverModel, ShowId};
pub use self::source::{Source, SourceId};

#[derive(Debug, Clone, PartialEq)]
pub enum IndexState<T, ID> {
    Index(T),
    Update((T, ID)),
    NotChanged,
}

pub(crate) trait Insert<T> {
    type Error;

    fn insert(&self) -> Result<T, Self::Error>;
}

pub trait Update<T, ID> {
    type Error;

    fn update(&self, _: ID) -> Result<T, Self::Error>;
}

// This might need to change in the future
pub trait Index<T, ID>: Insert<T> + Update<T, ID> {
    type Error;

    fn index(&self) -> Result<T, <Self as Index<T, ID>>::Error>;
}

/// FIXME: DOCS
pub trait Save<T> {
    /// The Error type to be returned.
    type Error;
    /// Helper method to easily save/"sync" current state of a diesel model to
    /// the Database.
    // TODO change this to save_to_connection and make a default impl for save() that opens the db/transaction
    fn save(&self) -> Result<T, Self::Error>;
}

/// Allows to use struct wrappers instead of i32 Id types.
#[macro_export]
macro_rules! make_id_wrapper {
    ($type_name:ident) => {
        use diesel::backend::Backend;
        use diesel::deserialize::{self, FromSql};
        use diesel::serialize::{self, Output, ToSql};
        use diesel::sql_types::Integer;
        use diesel::sqlite::Sqlite;
        #[derive(AsExpression, FromSqlRow, Debug, PartialEq, Eq, Hash, Clone, Copy, Default)]
        #[diesel(sql_type = diesel::sql_types::Integer)]
        pub struct $type_name(pub i32);

        impl<DB> FromSql<Integer, DB> for $type_name
        where
            DB: Backend,
            i32: FromSql<Integer, DB>,
        {
            fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
                i32::from_sql(bytes).map($type_name)
            }
        }

        impl ToSql<diesel::sql_types::Integer, Sqlite> for $type_name {
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
                <i32 as ToSql<Integer, Sqlite>>::to_sql(&self.0, out)
            }
        }
    };
}
