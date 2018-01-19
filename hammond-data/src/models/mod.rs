pub(crate) mod new_episode;
pub(crate) mod new_podcast;
pub(crate) mod new_source;

pub(crate) mod episode;
pub(crate) mod podcast;
pub(crate) mod source;

// use futures::prelude::*;
// use futures::future::*;

pub(crate) use self::new_episode::{NewEpisode, NewEpisodeMinimal};
pub(crate) use self::new_podcast::NewPodcast;
pub(crate) use self::new_source::NewSource;

pub use self::episode::{Episode, EpisodeMinimal, EpisodeWidgetQuery};
pub(crate) use self::episode::EpisodeCleanerQuery;
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
