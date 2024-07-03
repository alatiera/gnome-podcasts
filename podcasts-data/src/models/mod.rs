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

pub(crate) use self::discovery_settings::DiscoverySetting;

pub(crate) use self::new_episode::{NewEpisode, NewEpisodeMinimal};
pub(crate) use self::new_show::NewShow;
pub(crate) use self::new_source::NewSource;

#[cfg(test)]
pub(crate) use self::new_episode::NewEpisodeBuilder;
#[cfg(test)]
pub(crate) use self::new_show::NewShowBuilder;

pub use self::episode::{
    Episode, EpisodeCleanerModel, EpisodeId, EpisodeMinimal, EpisodeWidgetModel,
};
pub use self::show::{Show, ShowCoverModel};
pub use self::source::Source;

pub trait IdType {
    fn to_int(&self) -> i32;
}

impl IdType for i32 {
    fn to_int(&self) -> i32 {
        *self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndexState<T, ID: IdType> {
    Index(T),
    Update((T, ID)),
    NotChanged,
}

pub(crate) trait Insert<T> {
    type Error;

    fn insert(&self) -> Result<T, Self::Error>;
}

pub trait Update<T, ID: IdType> {
    type Error;

    fn update(&self, _: ID) -> Result<T, Self::Error>;
}

// This might need to change in the future
pub trait Index<T, ID: IdType>: Insert<T> + Update<T, ID> {
    type Error;

    fn index(&self) -> Result<T, <Self as Index<T, ID>>::Error>;
}

/// FIXME: DOCS
pub trait Save<T> {
    /// The Error type to be returned.
    type Error;
    /// Helper method to easily save/"sync" current state of a diesel model to
    /// the Database.
    fn save(&self) -> Result<T, Self::Error>;
}
