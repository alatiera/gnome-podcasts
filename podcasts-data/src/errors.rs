use diesel;
use diesel::r2d2;
use diesel_migrations::RunMigrationsError;
use http;
use hyper;
use native_tls;
use rss;
use url;
use xml;

use std::io;

use models::Source;

#[fail(
    display = "Request to {} returned {}. Context: {}",
    url, status_code, context
)]
#[derive(Fail, Debug)]
pub struct HttpStatusError {
    url: String,
    status_code: hyper::StatusCode,
    context: String,
}

impl HttpStatusError {
    pub fn new(url: String, code: hyper::StatusCode, context: String) -> Self {
        HttpStatusError {
            url,
            status_code: code,
            context,
        }
    }
}

#[derive(Fail, Debug)]
pub enum DataError {
    #[fail(display = "SQL Query failed: {}", _0)]
    DieselResultError(#[cause] diesel::result::Error),
    #[fail(display = "Database Migration error: {}", _0)]
    DieselMigrationError(#[cause] RunMigrationsError),
    #[fail(display = "R2D2 error: {}", _0)]
    R2D2Error(#[cause] r2d2::Error),
    #[fail(display = "R2D2 Pool error: {}", _0)]
    R2D2PoolError(#[cause] r2d2::PoolError),
    #[fail(display = "Hyper Error: {}", _0)]
    HyperError(#[cause] hyper::Error),
    #[fail(display = "ToStr Error: {}", _0)]
    HttpToStr(#[cause] http::header::ToStrError),
    #[fail(display = "Failed to parse a url: {}", _0)]
    UrlError(#[cause] url::ParseError),
    #[fail(display = "TLS Error: {}", _0)]
    TLSError(#[cause] native_tls::Error),
    #[fail(display = "IO Error: {}", _0)]
    IOError(#[cause] io::Error),
    #[fail(display = "RSS Error: {}", _0)]
    RssError(#[cause] rss::Error),
    #[fail(display = "XML Reader Error: {}", _0)]
    XmlReaderError(#[cause] xml::reader::Error),
    #[fail(display = "Error: {}", _0)]
    Bail(String),
    #[fail(display = "{}", _0)]
    HttpStatusGeneral(HttpStatusError),
    #[fail(display = "Source redirects to a new url")]
    FeedRedirect(Source),
    #[fail(display = "Feed is up to date")]
    FeedNotModified(Source),
    #[fail(display = "Error occured while Parsing an Episode. Reason: {}", reason)]
    ParseEpisodeError { reason: String, parent_id: i32 },
    #[fail(display = "Episode was not changed and thus skipped.")]
    EpisodeNotChanged,
}

impl From<RunMigrationsError> for DataError {
    fn from(err: RunMigrationsError) -> Self {
        DataError::DieselMigrationError(err)
    }
}

impl From<diesel::result::Error> for DataError {
    fn from(err: diesel::result::Error) -> Self {
        DataError::DieselResultError(err)
    }
}

impl From<r2d2::Error> for DataError {
    fn from(err: r2d2::Error) -> Self {
        DataError::R2D2Error(err)
    }
}

impl From<r2d2::PoolError> for DataError {
    fn from(err: r2d2::PoolError) -> Self {
        DataError::R2D2PoolError(err)
    }
}

impl From<hyper::Error> for DataError {
    fn from(err: hyper::Error) -> Self {
        DataError::HyperError(err)
    }
}

impl From<http::header::ToStrError> for DataError {
    fn from(err: http::header::ToStrError) -> Self {
        DataError::HttpToStr(err)
    }
}

impl From<url::ParseError> for DataError {
    fn from(err: url::ParseError) -> Self {
        DataError::UrlError(err)
    }
}

impl From<native_tls::Error> for DataError {
    fn from(err: native_tls::Error) -> Self {
        DataError::TLSError(err)
    }
}

impl From<io::Error> for DataError {
    fn from(err: io::Error) -> Self {
        DataError::IOError(err)
    }
}

impl From<rss::Error> for DataError {
    fn from(err: rss::Error) -> Self {
        DataError::RssError(err)
    }
}

impl From<xml::reader::Error> for DataError {
    fn from(err: xml::reader::Error) -> Self {
        DataError::XmlReaderError(err)
    }
}

impl From<String> for DataError {
    fn from(err: String) -> Self {
        DataError::Bail(err)
    }
}
