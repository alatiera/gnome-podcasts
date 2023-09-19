// pipeline.rs
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

// FIXME:
//! Docs.

use crate::downloader::client_builder;
use crate::errors::DataError;
use crate::Source;

/// The pipline to be run for indexing and updating a Podcast feed that originates from
/// `Source.uri`.
///
/// Messy temp diagram:
/// Source -> GET Request -> Update Etags -> Check Status -> Parse `xml/Rss` ->
/// Convert `rss::Channel` into `Feed` -> Index Podcast -> Index Episodes.
pub async fn pipeline<S>(sources: S) -> Result<(), reqwest::Error>
where
    S: IntoIterator<Item = Source>,
{
    let client = client_builder().build()?;

    let handles: Vec<_> = sources
        .into_iter()
        .map(|source| async {
            match source.into_feed(&client).await {
                Ok(feed) => match feed.index() {
                    Ok(_) => (),
                    Err(err) => error!(
                        "Error while indexing content feed into the database: {}",
                        err
                    ),
                },
                // Avoid spamming the stderr when it's not an actual error
                Err(DataError::FeedNotModified(_)) => (),
                Err(err) => error!("Error while fetching the latest xml feed: {}", err),
            }
        })
        .collect();
    futures::future::join_all(handles).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::truncate_db;
    use crate::dbqueries;
    use crate::Source;

    // (path, url) tuples.
    const URLS: &[&str] = &[
        "https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.\
         com/InterceptedWithJeremyScahill",
        "https://web.archive.org/web/20180120110314if_/https://feeds.feedburner.com/linuxunplugged",
        "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff",
        "https://web.archive.org/web/20180120104957if_/https://rss.art19.com/steal-the-stars",
        "https://web.archive.org/web/20180120104741if_/https://www.greaterthancode.\
         com/feed/podcast",
    ];

    #[test]
    /// Insert feeds and update/index them.
    fn test_pipeline() -> Result<(), DataError> {
        truncate_db()?;
        let bad_url = "https://gitlab.gnome.org/World/podcasts.atom";
        // if a stream returns error/None it stops
        // bad we want to parse all feeds regardless if one fails
        Source::from_url(bad_url)?;

        URLS.iter().for_each(|url| {
            // Index the urls into the source table.
            Source::from_url(url).unwrap();
        });

        let sources = dbqueries::get_sources()?;
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(sources))?;

        let sources = dbqueries::get_sources()?;
        // Run again to cover Unique constrains errors.
        rt.block_on(pipeline(sources))?;

        // Assert the index rows equal the controlled results
        assert_eq!(dbqueries::get_sources()?.len(), 6);
        assert_eq!(dbqueries::get_podcasts()?.len(), 5);
        assert_eq!(dbqueries::get_episodes()?.len(), 354);
        Ok(())
    }
}
