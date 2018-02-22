mod new_episode;
mod new_podcast;
mod new_source;

mod episode;
mod podcast;
mod source;

// use futures::prelude::*;
// use futures::future::*;

pub(crate) use self::episode::EpisodeCleanerQuery;
pub(crate) use self::new_episode::{NewEpisode, NewEpisodeMinimal};
pub(crate) use self::new_podcast::NewPodcast;
pub(crate) use self::new_source::NewSource;

#[cfg(test)]
pub(crate) use self::new_episode::NewEpisodeBuilder;
#[cfg(test)]
pub(crate) use self::new_podcast::NewPodcastBuilder;

pub use self::episode::{Episode, EpisodeMinimal, EpisodeWidgetQuery};
pub use self::podcast::{Podcast, PodcastCoverQuery};
pub use self::source::Source;

#[derive(Debug, Clone, PartialEq)]
pub enum IndexState<T> {
    Index(T),
    Update((T, i32)),
    NotChanged,
}

pub trait Insert<T, E> {
    fn insert(&self) -> Result<T, E>;
}

pub trait Update<T, E> {
    fn update(&self, i32) -> Result<T, E>;
}

// This might need to change in the future
pub trait Index<T, E>: Insert<T, E> + Update<T, E> {
    fn index(&self) -> Result<T, E>;
}

/// FIXME: DOCS
pub trait Save<T, E> {
    /// Helper method to easily save/"sync" current state of a diesel model to
    /// the Database.
    fn save(&self) -> Result<T, E>;
}
