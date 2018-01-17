pub(crate) mod new_episode;
pub(crate) mod new_podcast;
pub(crate) mod new_source;

pub(crate) mod episode;
pub(crate) mod podcast;
pub(crate) mod source;

use diesel::prelude::*;

pub(crate) use self::new_episode::{NewEpisode, NewEpisodeMinimal};
pub(crate) use self::new_podcast::NewPodcast;
pub(crate) use self::new_source::NewSource;

pub use self::episode::{Episode, EpisodeWidgetQuery};
pub(crate) use self::episode::EpisodeCleanerQuery;
pub use self::podcast::{Podcast, PodcastCoverQuery};
pub use self::source::Source;

#[allow(dead_code)]
enum IndexState<T> {
    Index(T),
    Update(T),
    NotChanged,
}

pub trait Insert {
    fn insert(&self, &SqliteConnection) -> QueryResult<usize>;
}

pub trait Update {
    fn update(&self, &SqliteConnection, i32) -> QueryResult<usize>;
}
