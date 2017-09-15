#![recursion_limit = "1024"]

extern crate structopt;
#[macro_use]
extern crate structopt_derive;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate reqwest;
#[macro_use]
extern crate diesel;

// use diesel::prelude::*;

pub mod cli;
pub mod schema;
pub mod models;

pub mod errors {

    use reqwest;
    use std::io;

    error_chain! {
        foreign_links {
            ReqError(reqwest::Error);
            IoError(io::Error);
            Log(::log::SetLoggerError);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
