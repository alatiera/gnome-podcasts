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

use crate::dbqueries;
use crate::errors::DataError;
use crate::models::Source;
use xml::{
    common::XmlVersion,
    reader,
    writer::{events::XmlEvent, EmitterConfig},
};

use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use std::fs::File;
// use std::io::BufReader;

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
// FIXME: Make it a Diesel model
/// Represents an `outline` xml element as per the `OPML` [specification][spec]
/// not `RSS` related sub-elements are omitted.
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

/// Export a file to `P`, taking the feeds from the database and outputting
/// them in opml format.
pub fn export_from_db<P: AsRef<Path>>(path: P, export_title: &str) -> Result<()> {
    let file = File::create(path)?;
    export_to_file(&file, export_title)
}

/// Export from `Source`s and `Show`s into `F` in OPML format
pub fn export_to_file<F: Write>(file: F, export_title: &str) -> Result<()> {
    let config = EmitterConfig::new().perform_indent(true);

    let mut writer = config.create_writer(file);

    let mut events: Vec<XmlEvent<'_>> = Vec::new();

    // Set up headers
    let doc = XmlEvent::StartDocument {
        version: XmlVersion::Version10,
        encoding: Some("UTF-8"),
        standalone: Some(false),
    };
    events.push(doc);

    let opml: XmlEvent<'_> = XmlEvent::start_element("opml")
        .attr("version", "2.0")
        .into();
    events.push(opml);

    let head: XmlEvent<'_> = XmlEvent::start_element("head").into();
    events.push(head);

    let title_ev: XmlEvent<'_> = XmlEvent::start_element("title").into();
    events.push(title_ev);

    let title_chars: XmlEvent<'_> = XmlEvent::characters(export_title).into();
    events.push(title_chars);

    // Close <title> & <head>
    events.push(XmlEvent::end_element().into());
    events.push(XmlEvent::end_element().into());

    let body: XmlEvent<'_> = XmlEvent::start_element("body").into();
    events.push(body);

    for event in events {
        writer.write(event)?;
    }

    // FIXME: Make this a model of a joined query (http://docs.diesel.rs/diesel/macro.joinable.html)
    let shows = dbqueries::get_podcasts()?.into_iter().map(|show| {
        let source = dbqueries::get_source_from_id(show.source_id()).unwrap();
        (source, show)
    });

    for (ref source, ref show) in shows {
        let title = show.title();
        let link = show.link();
        let xml_url = source.uri();

        let s_ev: XmlEvent<'_> = XmlEvent::start_element("outline")
            .attr("text", title)
            .attr("title", title)
            .attr("type", "rss")
            .attr("xmlUrl", xml_url)
            .attr("htmlUrl", link)
            .into();

        let end_ev: XmlEvent<'_> = XmlEvent::end_element().into();
        writer.write(s_ev)?;
        writer.write(end_ev)?;
    }

    // Close <body> and <opml>
    let end_bod: XmlEvent<'_> = XmlEvent::end_element().into();
    writer.write(end_bod)?;
    let end_opml: XmlEvent<'_> = XmlEvent::end_element().into();
    writer.write(end_opml)?;

    Ok(())
}

/// Extracts the `outline` elements from a reader `R` and returns a `HashSet` of `Opml` structs.
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
    use anyhow::Result;
    use chrono::Local;
    use futures::executor::block_on;

    use crate::database::{truncate_db, TEMPDIR};
    use crate::utils::get_feed;

    const URLS: &[(&str, &str)] = {
        &[
            (
                "tests/feeds/2018-01-20-Intercepted.xml",
                "https://web.archive.org/web/20180120083840if_/https://feeds.feedburner.\
                 com/InterceptedWithJeremyScahill",
            ),
            (
                "tests/feeds/2018-01-20-LinuxUnplugged.xml",
                "https://web.archive.org/web/20180120110314if_/https://feeds.feedburner.\
                 com/linuxunplugged",
            ),
            (
                "tests/feeds/2018-01-20-TheTipOff.xml",
                "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff",
            ),
            (
                "tests/feeds/2018-01-20-StealTheStars.xml",
                "https://web.archive.org/web/20180120104957if_/https://rss.art19.\
                 com/steal-the-stars",
            ),
            (
                "tests/feeds/2018-01-20-GreaterThanCode.xml",
                "https://web.archive.org/web/20180120104741if_/https://www.greaterthancode.\
                 com/feed/podcast",
            ),
            (
                "tests/feeds/2019-01-27-ACC.xml",
                "https://web.archive.org/web/20190127005213if_/https://anticapitalistchronicles.libsyn.com/rss"
            ),
        ]
    };

    #[test]
    fn test_extract() -> Result<()> {
        let int_title = String::from("Intercepted with Jeremy Scahill");
        let int_url = String::from("https://feeds.feedburner.com/InterceptedWithJeremyScahill");
        let int_desc = String::from(
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

    #[test]
    fn text_export() -> Result<()> {
        truncate_db()?;

        URLS.iter().for_each(|&(path, url)| {
            // Create and insert a Source into db
            let s = Source::from_url(url).unwrap();
            let feed = get_feed(path, s.id());
            block_on(feed.index()).unwrap();
        });

        let mut map: HashSet<Opml> = HashSet::new();
        let shows = dbqueries::get_podcasts()?.into_iter().map(|show| {
            let source = dbqueries::get_source_from_id(show.source_id()).unwrap();
            (source, show)
        });

        for (ref source, ref show) in shows {
            let title = show.title().to_string();
            // description is an optional field that we don't export
            let description = String::new();
            let url = source.uri().to_string();

            map.insert(Opml {
                title,
                description,
                url,
            });
        }

        let opml_path = TEMPDIR.path().join("podcasts.opml");
        export_from_db(opml_path.as_path(), "GNOME Podcasts Subscriptions")?;
        let opml_file = File::open(opml_path.as_path())?;
        assert_eq!(extract_sources(&opml_file)?, map);

        // extract_sources drains the reader its passed
        let mut opml_file = File::open(opml_path.as_path())?;
        let mut opml_str = String::new();
        opml_file.read_to_string(&mut opml_str)?;
        assert_eq!(opml_str, include_str!("../tests/export_test.opml"));
        Ok(())
    }
}
