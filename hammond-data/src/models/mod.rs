mod new_episode;
mod new_show;
mod new_source;

mod episode;
mod show;
mod source;

// use futures::prelude::*;
// use futures::future::*;

pub(crate) use self::episode::EpisodeCleanerQuery;
pub(crate) use self::new_episode::{NewEpisode, NewEpisodeMinimal};
pub(crate) use self::new_show::NewShow;
pub(crate) use self::new_source::NewSource;

#[cfg(test)]
pub(crate) use self::new_episode::NewEpisodeBuilder;
#[cfg(test)]
pub(crate) use self::new_show::NewShowBuilder;

pub use self::episode::{Episode, EpisodeMinimal, EpisodeWidgetQuery};
pub use self::show::{Show, ShowCoverQuery};
pub use self::source::Source;

#[derive(Debug, Clone, PartialEq)]
pub enum IndexState<T> {
    Index(T),
    Update((T, i32)),
    NotChanged,
}

pub trait Insert<T> {
    type Error;

    fn insert(&self) -> Result<T, Self::Error>;
}

pub trait Update<T> {
    type Error;

    fn update(&self, i32) -> Result<T, Self::Error>;
}

// This might need to change in the future
pub trait Index<T>: Insert<T> + Update<T> {
    type Error;

    fn index(&self) -> Result<T, <Self as Index<T>>::Error>;
}

/// FIXME: DOCS
pub trait Save<T> {
    /// The Error type to be returned.
    type Error;
    /// Helper method to easily save/"sync" current state of a diesel model to
    /// the Database.
    fn save(&self) -> Result<T, Self::Error>;
}
