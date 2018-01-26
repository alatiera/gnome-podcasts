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

use errors::*;

#[derive(Debug, Clone, PartialEq)]
pub enum IndexState<T> {
    Index(T),
    Update((T, i32)),
    NotChanged,
}

pub trait Insert {
    fn insert(&self) -> Result<()>;
}

pub trait Update {
    fn update(&self, i32) -> Result<()>;
}

pub trait Index: Insert + Update {
    fn index(&self) -> Result<()>;
}
