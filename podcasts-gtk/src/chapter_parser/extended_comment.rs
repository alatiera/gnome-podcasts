// extended_comment.rs
//
// Copyright 2025 nee <nee-git@patchouli.garden>
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

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::chapter_parser::Chapter;

/// Parses comment and writes updates into the chapters HashMap.
/// A chapter comment can either contain the title, or the timestamp.
/// So parsing of one chapter has to be done over multiple comment tags.
pub fn parse_extended_comment(
    chapters: &mut HashMap<String, Chapter>,
    comment: &str,
) -> Option<()> {
    //     "CHAPTER002NAME=Prologue"
    static RE_NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^CHAPTER([0-9]+)NAME=(.+)$").unwrap());
    let matches = RE_NAME.captures_iter(comment).next();
    if let Some(matches) = matches {
        let id = matches.get(1)?.as_str().to_string();
        let title = matches.get(2)?.as_str().to_string();
        let chapter = get_or_init(chapters, id.clone());
        chapter.id = id;
        chapter.title = title;
        return Some(());
    }

    //     "CHAPTER002=00:00:14.640"
    static RE_TIME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"CHAPTER([0-9]+)=([0-9]+):([0-9]+):([0-9\.]+)").unwrap());
    let matches = RE_TIME.captures_iter(comment).next();
    if let Some(matches) = matches {
        let id = matches.get(1)?.as_str().to_string();
        let hours = matches.get(2)?.as_str().parse::<i64>().ok()?;
        let minutes = matches.get(3)?.as_str().parse::<i64>().ok()?;
        let seconds = matches.get(4)?.as_str().parse::<f64>().ok()?;

        let chapter = get_or_init(chapters, id.clone());
        chapter.id = id;
        chapter.start = chrono::Duration::hours(hours)
            + chrono::Duration::minutes(minutes)
            + chrono::Duration::from_std(std::time::Duration::from_secs_f64(seconds)).ok()?;
        return Some(());
    }
    None
}

/// Gets an existing chapter from the hashmap, or initalizes a new one in it.
fn get_or_init(chapters: &mut HashMap<String, Chapter>, key: String) -> &mut Chapter {
    if chapters.contains_key(&key) {
        return chapters.get_mut(&key).unwrap();
    }
    chapters.insert(key.clone(), Chapter::default());
    chapters.get_mut(&key).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    #[rustfmt::skip]
    fn test_comment_newz() -> Result<()> {
        let comments = [
            "CHAPTER001=00:00:00.000",
            "CHAPTER001NAME=Intro",
            "CHAPTER002=00:00:14.640",
            "CHAPTER002NAME=Prologue",
            "CHAPTER003=00:03:47.967",
            "CHAPTER003NAME=US president accepts reality of election. Kind of.",
            "CHAPTER004=00:15:18.630",
            "CHAPTER004NAME=What damage can trump still cause",
            "CHAPTER005=00:22:11.746",
            "CHAPTER005NAME=Biden starts choosing his cabinet and announcing his policies",
            "CHAPTER006=00:33:43.575",
            "CHAPTER006NAME=Georgia Senate Runoffs",
            "CHAPTER007=00:45:02.492",
            "CHAPTER007NAME=Supremes make a covid-religion ruling where conservative judges show their stripes",
            "CHAPTER008=00:49:22.818",
            "CHAPTER008NAME=Countries struggles with consipiracy theories around Covid",
            "CHAPTER009=00:58:12.247",
            "CHAPTER009NAME=Poland and Hungary block EU's Gender Action Plan",
            "CHAPTER010=01:07:00.960",
            "CHAPTER010NAME=Brexit in the end stage",
            "CHAPTER011=01:16:29.643",
            "CHAPTER011NAME=Epilog",
            "CHAPTER012=01:17:06.308",
            "CHAPTER012NAME=Bonus Track",
        ];
        let expected_chapters = [
            Chapter {id: "001".to_string(),
                     title: "Intro".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(0, 0).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "002".to_string(),
                     title: "Prologue".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(14, 640000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "003".to_string(),
                     title: "US president accepts reality of election. Kind of.".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(227, 967000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "004".to_string(),
                     title: "What damage can trump still cause".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(918, 630000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "005".to_string(),
                     title: "Biden starts choosing his cabinet and announcing his policies".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(1331, 746000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "006".to_string(),
                     title: "Georgia Senate Runoffs".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(2023, 575000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "007".to_string(),
                     title: "Supremes make a covid-religion ruling where conservative judges show their stripes".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(2702, 492000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "008".to_string(),
                     title: "Countries struggles with consipiracy theories around Covid".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(2962, 818000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "009".to_string(),
                     title: "Poland and Hungary block EU's Gender Action Plan".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(3492, 247000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "010".to_string(),
                     title: "Brexit in the end stage".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(4020, 960000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "011".to_string(),
                     title: "Epilog".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(4589, 643000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "012".to_string(),
                     title: "Bonus Track".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(4626, 308000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            }
        ];
        let mut chapters_map: HashMap<String, Chapter> = HashMap::new();
        for s in comments {
            parse_extended_comment(&mut chapters_map, &s);
        }

        let mut result: Vec<_> = chapters_map.into_values().collect();
        result.sort_by_key(|c| c.id.clone());

        assert_eq!(expected_chapters.len(), result.len());

        let mut i = 0;
        for chapter in result {
            assert_eq!(expected_chapters[i], chapter);
            i = i+1;
        }
        Ok(())
    }
}
