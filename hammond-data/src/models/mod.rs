mod insertables;
mod queryables;

// Re-export the structs so the API doesn't change and brake everything else.
pub use self::queryables::{Episode, Podcast, Source};
pub use self::insertables::{NewEpisode, NewPodcast, NewSource};
