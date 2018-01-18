#![allow(unused_mut)]

use diesel;
use diesel::prelude::*;

use database::connection;
use dbqueries;
// use models::{Insert, Update};
use models::Insert;
use models::Source;
use schema::source;

use errors::*;

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

impl Insert for NewSource {
    fn insert(&self) -> Result<()> {
        use schema::source::dsl::*;
        let db = connection();
        let con = db.get()?;

        // FIXME: Insert or ignore
        let _ = diesel::insert_into(source).values(self).execute(&*con);
        Ok(())
    }
}

impl NewSource {
    pub(crate) fn new(uri: &str) -> NewSource {
        NewSource {
            uri: uri.trim().to_string(),
            last_modified: None,
            http_etag: None,
        }
    }

    // Look out for when tryinto lands into stable.
    pub(crate) fn into_source(self) -> Result<Source> {
        self.insert()?;
        dbqueries::get_source_from_uri(&self.uri)
    }
}
