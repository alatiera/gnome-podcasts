// mod.rs
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

mod extended_comment;
mod id3;

use anyhow::Result;
use gst::ClockTime;
use gst_pbutils::Discoverer;
use gst_pbutils::prelude::*;
use std::collections::HashMap;
use std::io::BufReader;

use crate::chapter_parser::extended_comment::parse_extended_comment;
use crate::chapter_parser::id3::read_id3_chapter;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub description: String,
    pub start: chrono::Duration,
    /// Currently unused, but might be useful for id3 Flag b,
    /// where chapters should be viewed as separated.
    #[allow(dead_code)]
    pub end: chrono::Duration,
}

/// Probes gst metadata for the uri and extracts Chapter tags.
pub fn load_chapters(uri: &str) -> Result<Vec<Chapter>> {
    let mut results = Vec::new();
    let timeout = ClockTime::try_from_seconds_f32(30.0)?;
    let discoverer = Discoverer::new(timeout)?;
    let info = discoverer.discover_uri(uri)?;

    for container in info.stream_list() {
        if let Some(tags) = container.tags() {
            for (name, tag) in tags.iter_generic() {
                results.append(&mut tag_generic(name, tag));
            }
        }
    }
    results.sort_by_key(|c| c.start);
    Ok(results)
}

/// Iterates over gst tags and turns them into Chapters.
fn tag_generic(name: &str, tags: gst::tags::GenericTagIter<'_>) -> Vec<Chapter> {
    let mut results = Vec::new();
    let mut extended_comment_chapters: HashMap<String, Chapter> = HashMap::new();
    for value in tags {
        if let Ok(s) = value.get::<String>() {
            if "extended-comment" == name {
                parse_extended_comment(&mut extended_comment_chapters, &s);
            }
        } else if "private-id3v2-frame" == name {
            if let Ok(sample) = value.get::<gst::sample::Sample>() {
                let readable = sample.buffer().map(|buffer| buffer.as_cursor_readable());
                if let Some(buf) = readable {
                    if let Ok(chapter) = read_id3_chapter(&mut BufReader::new(buf)) {
                        results.push(chapter);
                    }
                } else {
                    error!("cannot get buffer from gstSample");
                }
            } else {
                error!("cannot convert to gstSample");
            }
        }
    }
    let mut values: Vec<_> = extended_comment_chapters.into_values().collect();
    results.append(&mut values);
    results
}
