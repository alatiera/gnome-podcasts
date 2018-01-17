#![allow(unused_mut)]

use diesel::prelude::*;
use diesel;

use schema::source;
use models::Source;
use models::{Insert, Update};

use dbqueries;
use database::connection;

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
    fn insert(&self, con: &SqliteConnection) -> QueryResult<usize> {
        use schema::source::dsl::*;
        diesel::insert_into(source).values(self).execute(&*con)
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

    fn index(&self) -> Result<()> {
        let db = connection();
        let con = db.get()?;

        // Throw away the result like `insert or ignore`
        // Diesel deos not support `insert or ignore` yet.
        let _ = self.insert(&con);
        Ok(())
    }

    // Look out for when tryinto lands into stable.
    pub(crate) fn into_source(self) -> Result<Source> {
        self.index()?;
        dbqueries::get_source_from_uri(&self.uri)
    }
}
