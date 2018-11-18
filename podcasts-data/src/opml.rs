// opml.rs
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

//! FIXME: Docs

// #![allow(unused)]

use crate::errors::DataError;
use crate::models::Source;
use xml::reader;

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::Path;

// use std::fs::{File, OpenOptions};
// use std::io::BufReader;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
// FIXME: Make it a Diesel model
/// Represents an `outline` xml element as per the `OPML` [specification][spec]
/// not `RSS` related sub-elements are ommited.
///
/// [spec]: http://dev.opml.org/spec2.html
pub struct Opml {
    title: String,
    description: String,
    url: String,
}

/// Import feed url's from a `R` into the `Source` table.
// TODO: Write test
pub fn import_to_db<R: Read>(reader: R) -> Result<Vec<Source>, reader::Error> {
    let feeds = extract_sources(reader)?
        .iter()
        .map(|opml| Source::from_url(&opml.url))
        .filter_map(|s| {
            if let Err(ref err) = s {
                let txt = "If you think this might be a bug please consider filling a report over \
                           at https://gitlab.gnome.org/World/podcasts/issues/new";

                error!("Failed to import a Show: {}", err);
                error!("{}", txt);
            }

            s.ok()
        })
        .collect();

    Ok(feeds)
}

/// Open a File from `P`, try to parse the OPML then insert the Feeds in the database and
/// return the new `Source`s
// TODO: Write test
pub fn import_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Source>, DataError> {
    let content = fs::read(path)?;
    import_to_db(content.as_slice()).map_err(From::from)
}

/// Extracts the `outline` elemnts from a reader `R` and returns a `HashSet` of `Opml` structs.
pub fn extract_sources<R: Read>(reader: R) -> Result<HashSet<Opml>, reader::Error> {
    let mut list = HashSet::new();
    let parser = reader::EventReader::new(reader);

    parser
        .into_iter()
        .map(|e| match e {
            Ok(reader::XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                if name.local_name == "outline" {
                    let mut title = String::new();
                    let mut url = String::new();
                    let mut description = String::new();

                    attributes.into_iter().for_each(|attribute| {
                        match attribute.name.local_name.as_str() {
                            "title" => title = attribute.value,
                            "xmlUrl" => url = attribute.value,
                            "description" => description = attribute.value,
                            _ => {}
                        }
                    });

                    let feed = Opml {
                        title,
                        description,
                        url,
                    };
                    list.insert(feed);
                }
                Ok(())
            }
            Err(err) => Err(err),
            _ => Ok(()),
        })
        .collect::<Result<Vec<_>, reader::Error>>()?;

    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use failure::Error;

    #[test]
    fn test_extract() -> Result<(), Error> {
        let int_title = String::from("Intercepted with Jeremy Scahill");
        let int_url = String::from("https://feeds.feedburner.com/InterceptedWithJeremyScahill");
        let int_desc =
            String::from(
                "The people behind The Intercept’s fearless reporting and incisive \
                 commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and others—discuss the \
                 crucial issues of our time: national security, civil liberties, foreign policy, \
                 and criminal justice.  Plus interviews with artists, thinkers, and newsmakers \
                 who challenge our preconceptions about the world we live in.",
            );

        let dec_title = String::from("Deconstructed with Mehdi Hasan");
        let dec_url = String::from("https://rss.prod.firstlook.media/deconstructed/podcast.rss");
        let dec_desc = String::from(
            "Journalist Mehdi Hasan is known around the world for his televised takedowns of \
             presidents and prime ministers. In this new podcast from The Intercept, Mehdi \
             unpacks a game-changing news event of the week while challenging the conventional \
             wisdom. As a Brit, a Muslim and an immigrant based in Donald Trump's Washington \
             D.C., Mehdi gives a refreshingly provocative perspective on the ups and downs of \
             American—and global—politics.",
        );

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let sample1 = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?> \
             <opml version=\"2.0\"> \
               <head> \
                 <title>Test OPML File</title> \
                 <dateCreated>{}</dateCreated> \
                 <docs>http://www.opml.org/spec2</docs> \
               </head> \
               <body> \
                 <outline type=\"rss\" title=\"{}\" description=\"{}\" xmlUrl=\"{}\"/> \
                 <outline type=\"rss\" title=\"{}\" description=\"{}\" xmlUrl=\"{}\"/> \
               </body> \
             </opml>",
            Local::now().format("%a, %d %b %Y %T %Z"),
            int_title,
            int_desc,
            int_url,
            dec_title,
            dec_desc,
            dec_url,
        );

        let map = hashset![
            Opml {
                title: int_title,
                description: int_desc,
                url: int_url
            },
            Opml {
                title: dec_title,
                description: dec_desc,
                url: dec_url
            },
        ];
        assert_eq!(extract_sources(sample1.as_bytes())?, map);
        Ok(())
    }
}
