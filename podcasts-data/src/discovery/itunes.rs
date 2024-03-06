// itunes.rs
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

use crate::discovery::data::*;
use anyhow::Result;
use chrono::prelude::*;
use serde::Deserialize;
use url::Url;

// curl "https://itunes.apple.com/search?term=chapo&entity=podcast&limit=10"
#[derive(Deserialize)]
struct SearchResult {
    results: Vec<Podcast>,
}

#[derive(Deserialize)]
struct Podcast {
    #[serde(rename = "feedUrl")]
    feed_url: UrlString,
    #[serde(rename = "collectionName")]
    collection_name: String, // Name of the podcast
    #[serde(rename = "artistName")]
    artist_name: String,
    #[serde(rename = "releaseDate")]
    release_date: Option<DateTime<Local>>,
    #[serde(rename = "trackCount")]
    track_count: i32,
    #[serde(rename = "artworkUrl100")]
    artwork_url_100: UrlString,
}

impl From<Podcast> for FoundPodcast {
    fn from(p: Podcast) -> FoundPodcast {
        FoundPodcast {
            feed: p.feed_url,
            title: p.collection_name,
            author: p.artist_name,
            description: "".to_string(),
            art: p.artwork_url_100,
            episode_count: Some(p.track_count),
            last_publication: p.release_date,
        }
    }
}

pub async fn search(query: &str, enabled: bool) -> Result<Vec<FoundPodcast>> {
    if !enabled {
        return Ok(vec![]);
    }
    let url = Url::parse_with_params(
        "https://itunes.apple.com/search?entity=podcast&limit=10",
        &[("term", query)],
    )?;
    let client = crate::downloader::client_builder().build()?;
    let result: SearchResult = client.get(url).send().await?.json().await?;
    Ok(result.results.into_iter().map(|p| p.into()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/itunes/search_1.txt")?;
        let result: SearchResult = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.results.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![
            FoundPodcast {
                feed: "https://anchor.fm/s/19346b24/podcast/rss".to_string(),
                title: "CushVlogs Audio - Matt Christman - Chapo Trap House".to_string(),
                author: "Jackson Jacker".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts124/v4/74/41/7c/74417c8a-151f-5090-5460-fe5e7ca0e671/mza_7528059777000521713.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(167),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2021-06-08T06:56:00+02:00").unwrap().with_timezone(&chrono::Local))
            }
        ];
        assert_eq!(1, found.len());
        assert_eq!(expected, found);
        Ok(())
    }

    #[test]
    fn test_10_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/itunes/search_10.txt")?;
        let result: SearchResult = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.results.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![
            FoundPodcast {
                feed: "https://feeds.acast.com/public/shows/7540dfb3-3c5f-43ae-91d3-aad22b2ede46".to_string(),
                title: "Chapo".to_string(),
                author: "VICE".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts116/v4/58/9b/8d/589b8d72-13aa-55cb-463f-ad41096ca9ea/mza_560982554884614556.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(17),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2023-01-25T06:01:00+01:00").unwrap().with_timezone(&chrono::Local))
            }, FoundPodcast {
                feed: "https://feeds.soundcloud.com/users/soundcloud:users:211911700/sounds.rss".to_string(),
                title: "Chapo Trap House".to_string(),
                author: "Chapo Trap House".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts18/v4/37/d9/a0/37d9a0b4-64f8-70d4-722e-b3a8772f3424/mza_5335192259855381405.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(507),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2024-03-05T21:55:00+01:00").unwrap().with_timezone(&chrono::Local))
            }, FoundPodcast {
                feed: "https://feeds.megaphone.fm/WMHY2717952910".to_string(),
                title: "El Chapo: Dos rostros de un capo Podcast".to_string(),
                author: "CNN en Español".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts122/v4/31/e9/bb/31e9bb57-31f8-447d-e619-bf94e2ee42af/mza_7685287012963567517.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(7),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2020-07-15T11:10:00+02:00").unwrap().with_timezone(&chrono::Local))
            }, FoundPodcast {
                feed: "https://anchor.fm/s/19346b24/podcast/rss".to_string(),
                title: "CushVlogs Audio - Matt Christman - Chapo Trap House".to_string(),
                author: "Jackson Jacker".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts124/v4/74/41/7c/74417c8a-151f-5090-5460-fe5e7ca0e671/mza_7528059777000521713.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(167),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2021-06-08T06:56:00+02:00").unwrap().with_timezone(&chrono::Local))
            }, FoundPodcast {
                feed: "https://audioboom.com/channels/4905580.rss".to_string(),
                title: "‘El Chapo’: ¿héroe o villano?".to_string(),
                author: "Univision".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts116/v4/57/34/8e/57348e38-6c9a-7579-8269-42e61786ae3b/mza_17242328811639650011.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(4),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2017-05-23T02:59:00+02:00").unwrap().with_timezone(&chrono::Local))
            }, FoundPodcast {
                feed: "https://anchor.fm/s/5b423b40/podcast/rss".to_string(),
                title: "Chapo".to_string(),
                author: "Jack Gold".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts125/v4/43/67/d3/4367d309-707e-6b7e-390a-202565c1d530/mza_9950196886709505935.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(1),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2021-05-15T07:32:00+02:00").unwrap().with_timezone(&chrono::Local))
            }, FoundPodcast {
                feed: "https://anchor.fm/s/1e56647c/podcast/rss".to_string(),
                title: "Chapo".to_string(),
                author: "Mijo 713".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts123/v4/b5/7c/41/b57c410f-8b2f-d9ce-df76-44b5947ea261/mza_17707437198336862.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(1),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2020-04-25T21:46:00+02:00").unwrap().with_timezone(&chrono::Local))
            }, FoundPodcast {
                feed: "https://www.omnycontent.com/d/playlist/e73c998e-6e60-432f-8610-ae210140c5b1/a6b57093-81fe-4401-a963-af2000fe4e3a/56b7926c-6bfd-4d8c-81f9-af2000ffad49/podcast.rss".to_string(),
                title: "Surviving El Chapo: The Twins Who Brought Down A Drug Lord".to_string(),
                author: "iHeartPodcasts".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts116/v4/aa/90/0f/aa900f15-7a67-e498-7b8c-a8efc9a667b5/mza_18039635476170983620.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(26),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2023-12-06T09:00:00+01:00").unwrap().with_timezone(&chrono::Local))
            }, FoundPodcast {
                feed: "https://anchor.fm/s/3a15ad8/podcast/rss".to_string(),
                title: "CHAPOS Corner".to_string(),
                author: "Chapo".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts125/v4/a5/08/da/a508da5c-09c7-bc4d-3c2d-938bf406b36b/mza_14501171467539691063.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(704),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2022-10-19T02:24:00+02:00").unwrap().with_timezone(&chrono::Local))
            }, FoundPodcast {
                feed: "https://anchor.fm/s/5d83aab0/podcast/rss".to_string(),
                title: "EL CHAPO".to_string(),
                author: "Bernardo".to_string(),
                description: "".to_string(),
                art: "https://is1-ssl.mzstatic.com/image/thumb/Podcasts115/v4/df/3b/2c/df3b2cbc-a8ac-315b-d5db-56d055690dfc/mza_8387652467730745876.jpg/100x100bb.jpg".to_string(),
                episode_count: Some(10),
                last_publication: Some(chrono::DateTime::parse_from_rfc3339("2021-06-22T23:50:00+02:00").unwrap().with_timezone(&chrono::Local))
            }
        ];
        assert_eq!(10, found.len());
        assert_eq!(expected, found);
        Ok(())
    }

    #[test]
    fn empty_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/itunes/search_empty.txt")?;
        let result: SearchResult = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.results.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![];
        assert_eq!(expected, found);
        Ok(())
    }

    #[test]
    fn unicode_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/itunes/search_unicode.txt")?;
        let result: SearchResult = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.results.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![FoundPodcast {
            feed: "https://feeds.fireside.fm/cornerspaeti/rss".to_string(),
            title: "Corner Späti".to_string(),
            author: "The Späti Boys".to_string(),
            description: "".to_string(),
            art: "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg"
                .to_string(),
            episode_count: Some(348),
            last_publication: Some(
                chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
            ),
        }];
        assert_eq!(expected, found);
        Ok(())
    }
}
