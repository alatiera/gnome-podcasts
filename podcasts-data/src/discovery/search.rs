// platform.rs
//
// Copyright 2022-2024 nee <nee-git@patchouli.garden>
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

use crate::dbqueries;
use crate::discovery::data::*;
use crate::discovery::fyyd;
use crate::discovery::itunes;
use anyhow::Result;
use tokio::join;

/// Sends a http search to all platforms that are active in the settings.
/// It joins all results into a Vector and tries to filter out duplicates.
/// Results are sorted as they are returned by the Search platforms.
pub async fn search(query: &str) -> Result<Vec<FoundPodcast>> {
    // This looks like it could be abstracted more,
    // but traits with async fns are impossible to deal with.
    let settings = dbqueries::get_discovery_settings();

    let fyyd = fyyd::search(query, *settings.get("fyyd.de").unwrap_or(&false));
    let itunes = itunes::search(query, *settings.get("itunes.apple.com").unwrap_or(&false));
    let (fyyd, itunes) = join!(fyyd, itunes);

    let fyyd: Vec<FoundPodcast> = fyyd.map_err(|e| error!("fyyd {e}")).unwrap_or_default();
    let itunes: Vec<FoundPodcast> = itunes.map_err(|e| error!("itunes {e}")).unwrap_or_default();

    trace!("combining {fyyd:#?} with {itunes:#?}");
    let mut merged = fyyd;
    merge_results(&mut merged, itunes);
    Ok(merged)
}

fn merge_results(merged: &mut Vec<FoundPodcast>, other_results: Vec<FoundPodcast>) {
    for p in other_results.into_iter() {
        if let Some(existing) = merged.iter_mut().find(|p2| p.eq(*p2)) {
            existing.combine(p);
        } else {
            merged.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge() -> Result<()> {
        let itunes = vec![FoundPodcast {
            feed: "https://feeds.fireside.fm/cornerspaeti/rss".to_string(),
            title: "Corner Späti".to_string(),
            author: "The Späti Boys".to_string(),
            description: "".to_string(),
            art: "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg"
                .to_string(),
            episode_count: Some(347),
            last_publication: Some(
                chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
            ),
        }];
        let fyyd = vec![FoundPodcast {
            feed: "https://feeds.fireside.fm/cornerspaeti/rss".to_string(),
            title: "Corner Späti".to_string(),
            author: "The Späti Boys".to_string(),
            description: "Weekly discussions of a deteriorating world all from the comfort of your local smoke-filled Spätkauf.\nhttps://www.patreon.com/cornerspaeti\nhttps://www.operationglad.io/start\n".to_string(),
            art: "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg".to_string(),
            episode_count: Some(348),
            last_publication: Some(chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00").unwrap().with_timezone(&chrono::Local))
        }];
        let expected: Vec<FoundPodcast> = vec![
            FoundPodcast {
                feed: "https://feeds.fireside.fm/cornerspaeti/rss".to_string(),
                title: "Corner Späti".to_string(),
                author: "The Späti Boys".to_string(),
                description: "Weekly discussions of a deteriorating world all from the comfort of your local smoke-filled Spätkauf.\nhttps://www.patreon.com/cornerspaeti\nhttps://www.operationglad.io/start\n".to_string(),
                art: "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg".to_string(),
                episode_count: Some(348),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00").unwrap().with_timezone(&chrono::Local))
            }
        ];
        let itunes2 = itunes.clone();
        let mut merged = itunes;
        merge_results(&mut merged, fyyd);
        assert_eq!(expected, merged);
        merge_results(&mut merged, itunes2);
        assert_eq!(expected, merged);
        Ok(())
    }

    #[test]
    fn merge_nones() -> Result<()> {
        let itunes = vec![FoundPodcast {
            feed: "https://feeds.fireside.fm/cornerspaeti/rss".to_string(),
            title: "Corner Späti".to_string(),
            author: "The Späti Boys".to_string(),
            description: "".to_string(),
            art: "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg"
                .to_string(),
            episode_count: None,
            last_publication: None,
        }];
        let fyyd = vec![FoundPodcast {
            feed: "https://feeds.fireside.fm/cornerspaeti/rss".to_string(),
            title: "Corner Späti".to_string(),
            author: "The Späti Boys".to_string(),
            description: "Weekly discussions of a deteriorating world all from the comfort of your local smoke-filled Spätkauf.\nhttps://www.patreon.com/cornerspaeti\nhttps://www.operationglad.io/start\n".to_string(),
            art: "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg".to_string(),
            episode_count: Some(348),
            last_publication: Some(chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00").unwrap().with_timezone(&chrono::Local))
        }];
        let expected: Vec<FoundPodcast> = vec![
            FoundPodcast {
                feed: "https://feeds.fireside.fm/cornerspaeti/rss".to_string(),
                title: "Corner Späti".to_string(),
                author: "The Späti Boys".to_string(),
                description: "Weekly discussions of a deteriorating world all from the comfort of your local smoke-filled Spätkauf.\nhttps://www.patreon.com/cornerspaeti\nhttps://www.operationglad.io/start\n".to_string(),
                art: "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg".to_string(),
                episode_count: Some(348),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00").unwrap().with_timezone(&chrono::Local))
            }
        ];
        let itunes2 = itunes.clone();
        let mut merged = itunes;
        merge_results(&mut merged, fyyd);
        assert_eq!(expected, merged);
        merge_results(&mut merged, itunes2);
        assert_eq!(expected, merged);
        Ok(())
    }
}
