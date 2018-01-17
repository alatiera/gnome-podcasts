pub(crate) mod queryables;
pub(crate) mod new_episode;
pub(crate) mod new_podcast;
pub(crate) mod new_source;

use diesel::prelude::*;

pub(crate) use self::new_episode::{NewEpisode, NewEpisodeMinimal};
pub(crate) use self::new_podcast::NewPodcast;
pub(crate) use self::new_source::NewSource;

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
