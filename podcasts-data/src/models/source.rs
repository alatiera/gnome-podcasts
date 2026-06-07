// source.rs
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

use base64::engine::general_purpose;
use base64::prelude::*;
use diesel::SaveChangesDsl;
use http::StatusCode;
use http::header::{
    AUTHORIZATION, ETAG, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED, LOCATION,
};
use rss::Channel;
use std::str::FromStr;
use url::Url;

use crate::database::connection;
use crate::errors::*;
use crate::feed::{Feed, FeedBuilder};
use crate::http::RetryContext;
use crate::make_id_wrapper;
use crate::models::{NewSource, Save};
use crate::schema::source;

make_id_wrapper!(SourceId);

#[derive(Queryable, Identifiable, AsChangeset, PartialEq, Selectable)]
#[diesel(table_name = source)]
#[diesel(treat_none_as_null = true)]
#[derive(Debug, Clone)]
/// Diesel Model of the source table.
pub struct Source {
    id: SourceId,
    uri: String,
    last_modified: Option<String>,
    http_etag: Option<String>,
}

impl Save<Source> for Source {
    type Error = DataError;

    /// Helper method to easily save/"sync" current state of self to the
    /// Database.
    fn save(&self) -> Result<Source, Self::Error> {
        let db = connection();
        let mut con = db.get()?;

        self.save_changes::<Source>(&mut con).map_err(From::from)
    }
}

impl Source {
    /// Get the source `id` column.
    pub fn id(&self) -> SourceId {
        self.id
    }

    /// Represents the location(usually url) of the Feed xml file.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Set the `uri` field value.
    pub fn set_uri(&mut self, uri: String) {
        self.uri = uri;
    }

    /// Represents the Http Last-Modified Header field.
    ///
    /// See [RFC 7231](https://tools.ietf.org/html/rfc7231#section-7.2) for more.
    pub fn last_modified(&self) -> Option<&str> {
        self.last_modified.as_deref()
    }

    /// Set `last_modified` value.
    pub fn set_last_modified(&mut self, value: Option<String>) {
        // self.last_modified = value.map(|x| x.to_string());
        self.last_modified = value;
    }

    /// Represents the Http Etag Header field.
    ///
    /// See [RFC 7231](https://tools.ietf.org/html/rfc7231#section-7.2) for more.
    pub fn http_etag(&self) -> Option<&str> {
        self.http_etag.as_deref()
    }

    /// Set `http_etag` value.
    pub fn set_http_etag(&mut self, value: Option<&str>) {
        self.http_etag = value.map(|x| x.to_string());
    }

    /// Extract Etag and LastModifier from res, and update self and the
    /// corresponding db row.
    fn update_etag(mut self, res: &reqwest::Response) -> Result<Self, DataError> {
        let headers = res.headers();

        let etag = headers.get(ETAG).and_then(|h| h.to_str().ok());
        let lmod = headers
            .get(LAST_MODIFIED)
            .and_then(|h| h.to_str().ok())
            .map(From::from);

        if (self.http_etag() != etag) || (self.last_modified != lmod) {
            self.set_http_etag(etag);
            self.set_last_modified(lmod);
            self = self.save()?;
        }

        Ok(self)
    }

    /// Clear the `HTTP` `Etag` and `Last-modified` headers.
    /// This method does not sync the state of self in the database, call
    /// .save() method explicitly
    fn clear_etags(&mut self) {
        debug!("Source etags before clear: {:#?}", &self);
        self.http_etag = None;
        self.last_modified = None;
    }

    fn make_err(self, context: &str, code: StatusCode) -> DataError {
        DataError::HttpStatusGeneral {
            url: self.uri,
            status_code: code,
            context: context.into(),
        }
    }

    // TODO match on more stuff
    // 301: Moved Permanently
    // 304: Up to date Feed, checked with the Etag
    // 307: Temporary redirect of the url
    // 308: Permanent redirect of the url
    // 401: Unathorized
    // 403: Forbidden
    // 408: Timeout
    // 410: Feed deleted
    // TODO: Rething this api,
    fn match_status(mut self, res: reqwest::Response) -> Result<reqwest::Response, DataError> {
        let code = res.status();

        if code.is_success() {
            // If request is successful save the etag
            self = self.update_etag(&res)?
        } else {
            match code.as_u16() {
                // Save etags if it returns NotModified
                304 => self = self.update_etag(&res)?,
                // Clear the Etag/lmod else
                _ => {
                    self.clear_etags();
                    self = self.save()?;
                }
            };
        };

        match code.as_u16() {
            304 => {
                info!("304: Source, {} is up to date", self.uri());
                return Err(DataError::FeedNotModified(self));
            }
            301 | 308 => {
                info!("Feed was moved permanently.");
                self = self.update_url(&res)?;
                return Err(DataError::FeedRedirect(self));
            }
            302 | 307 => {
                info!("302/307: Temporary Redirect.");
                return Err(DataError::FeedRedirect(self));
            }
            401 => return Err(self.make_err("401: Unauthorized.", code)),
            403 => return Err(self.make_err("403: Forbidden.", code)),
            404 => return Err(self.make_err("404: Not found.", code)),
            408 => return Err(self.make_err("408: Request Timeout.", code)),
            410 => return Err(self.make_err("410: Feed was deleted..", code)),
            _ => info!("HTTP StatusCode: {}", code),
        };

        Ok(res)
    }

    fn update_url(mut self, res: &reqwest::Response) -> Result<Self, DataError> {
        let code = res.status();
        let headers = res.headers();
        info!("HTTP StatusCode: {}", code);
        debug!("Headers {:#?}", headers);

        if let Some(new_url) = headers.get(LOCATION) {
            debug!("Previous Source: {:#?}", &self);
            let old_url = self.uri.clone();
            let new_url: String = new_url.to_str()?.into();

            self.set_uri(new_url.clone());
            self.clear_etags();
            self = self.save()?;

            if let Err(err) = crate::sync::Show::store_by_uri(
                old_url.clone(),
                crate::sync::ShowAction::Moved(new_url.clone()),
            ) {
                error!(
                    "Failed to store sync item for Podcast URL Move {old_url} - to - {new_url}: {err}"
                );
            }

            debug!("Updated Source: {:#?}", &self);
            info!(
                "Feed url of Source {}, was updated successfully.",
                self.uri()
            );
        }

        Ok(self)
    }

    /// Construct a new `Source` with the given `uri` and index it.
    ///
    /// This only indexes the `Source` struct, not the Podcast Feed.
    pub fn from_url(uri: &str) -> Result<Source, DataError> {
        let url = Url::parse(uri)?;

        NewSource::new(&url).to_source()
    }

    /// `Feed` constructor.
    ///
    /// Fetches the latest xml Feed.
    ///
    /// Updates the validator Http Headers.
    ///
    /// Consumes `self` and Returns the corresponding `Feed` Object.
    // Refactor into TryInto once it lands on stable.
    pub async fn into_feed(self) -> Result<Feed, DataError> {
        let id = self.id();

        let resp = self.get_response().await?;
        let chan = response_to_channel(resp).await?;

        FeedBuilder::default()
            .channel(chan)
            .source_id(id)
            .build()
            .map_err(|err| DataError::BuilderError(format!("{err}")))
    }

    async fn get_response(self) -> Result<reqwest::Response, DataError> {
        let mut retry_context = RetryContext::default();
        let mut source = self;
        loop {
            match source.send_request(&mut retry_context).await {
                Ok(response) => return Ok(response),
                Err(err) => match err {
                    DataError::FeedRedirect(s) => {
                        info!("Following redirect...");
                        source = s;
                    }
                    e => return Err(e),
                },
            }
        }
    }

    /// adds auth/cache headers
    fn add_request_headers(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Ok(url) = Url::parse(self.uri())
            && let Some(password) = url.password()
        {
            let mut auth = "Basic ".to_owned();
            auth.push_str(&general_purpose::URL_SAFE.encode(
                //url.username() converts @ symbols to %40 automatically.  The "replace" undoes that.
                format!("{}:{}", url.username().replace("%40", "@"), password),
            ));
            req = req.header(AUTHORIZATION, HeaderValue::from_str(&auth).unwrap());
        }

        if let Some(etag) = self.http_etag() {
            req = req.header(IF_NONE_MATCH, HeaderValue::from_str(etag).unwrap());
        }

        if let Some(lmod) = self.last_modified() {
            req = req.header(IF_MODIFIED_SINCE, HeaderValue::from_str(lmod).unwrap());
        }
        req
    }

    async fn send_request(
        self,
        retry_context: &mut RetryContext,
    ) -> Result<reqwest::Response, DataError> {
        let uri = Url::from_str(self.uri())?;
        let res = retry_context
            .prepared_send(uri, |req| self.add_request_headers(req))
            .await?;
        self.match_status(res)
    }
}

async fn response_to_channel(res: reqwest::Response) -> Result<Channel, DataError> {
    use bytes::buf::Buf;
    let chunk = res.bytes().await?;

    // Channel will do it's own decoding of strings
    // based on what is specified in <?xml encoding="..."?>.
    // So just pass it the raw byets.
    Channel::read_from(chunk.reader()).map_err(From::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    use crate::database::reset_db;
    use crate::dbqueries;
    use crate::test_feeds::*;
    use crate::utils::get_feed;

    #[test]
    fn test_into_feed() -> Result<()> {
        let _tempfile = reset_db()?;

        let rt = tokio::runtime::Runtime::new()?;

        let server = mock_feed_server()?;
        let feed_url = mock_feed_url(&server, MOCK_FEED_INTERCEPTED);
        let source = Source::from_url(&feed_url)?;
        let id = source.id();
        let feed = source.into_feed();
        let feed = rt.block_on(feed)?;

        let expected = get_feed("tests/feeds/2018-01-20-Intercepted.xml", id);
        assert_eq!(expected, feed);
        Ok(())
    }

    #[test]
    fn test_into_non_utf8() -> Result<()> {
        let _tempfile = reset_db()?;

        let rt = tokio::runtime::Runtime::new()?;

        let server = unsafe { iso_encoded_mock_server() }?;
        let feed_url = mock_feed_url(&server, MOCK_FEED_SERIES_I_CINEMA);
        let source = Source::from_url(&feed_url)?;
        let id = source.id();
        let feed = source.into_feed();
        let feed = rt.block_on(feed)?;

        let expected = get_feed("tests/feeds/2022-series-i-cinema.xml", id);
        assert_eq!(expected, feed);

        feed.index()?;
        assert_eq!(dbqueries::get_podcasts()?.len(), 1);
        assert_eq!(
            dbqueries::get_podcasts()?[0].description(),
            "Els clàssics, les novetats de la cartellera i les millors \
                    sèries, tot en un sol podcast."
        );
        Ok(())
    }
}
