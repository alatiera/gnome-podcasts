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
