// new_source.rs
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


use diesel;
use diesel::prelude::*;
use url::Url;

use database::connection;
use dbqueries;
// use models::{Insert, Update};
use errors::DataError;
use models::Source;
use schema::source;

#[derive(Insertable)]
#[table_name = "source"]
#[derive(Debug, Clone, Default, Builder, PartialEq)]
#[builder(default)]
#[builder(derive(Debug))]
#[builder(setter(into))]
pub(crate) struct NewSource {
    uri: String,
    last_modified: Option<String>,
    http_etag: Option<String>,
}

impl NewSource {
    pub(crate) fn new(uri: &Url) -> NewSource {
        NewSource {
            uri: uri.to_string(),
            last_modified: None,
            http_etag: None,
        }
    }

    pub(crate) fn insert_or_ignore(&self) -> Result<(), DataError> {
        use schema::source::dsl::*;
        let db = connection();
        let con = db.get()?;

        diesel::insert_or_ignore_into(source)
            .values(self)
            .execute(&con)
            .map(|_| ())
            .map_err(From::from)
    }

    // Look out for when tryinto lands into stable.
    pub(crate) fn to_source(&self) -> Result<Source, DataError> {
        self.insert_or_ignore()?;
        dbqueries::get_source_from_uri(&self.uri).map_err(From::from)
    }
}
