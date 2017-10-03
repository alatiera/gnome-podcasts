extern crate log;
extern crate loggerv;

extern crate structopt;
#[macro_use]
extern crate structopt_derive;

#[macro_use]
extern crate error_chain;

extern crate hammond;

use structopt::StructOpt;
use hammond::errors::*;
use hammond::downloader;
use hammond::index_feed;
use hammond::dbqueries;

// Should probably had made an Enum instead.
#[derive(StructOpt, Debug)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Enable logging, use multiple `v`s to increase verbosity
    #[structopt(short = "v", long = "verbose")]
    verbosity: u64,

    #[structopt(long = "update")] up: bool,

    #[structopt(long = "latest")] latest: bool,

    #[structopt(long = "download", default_value = "-1")] dl: i64,

    #[structopt(short = "a", long = "add", default_value = "")] add: String,
}

fn run() -> Result<()> {
    let args = Opt::from_args();

    loggerv::init_with_verbosity(args.verbosity)?;

    hammond::init()?;

    // Initial prototype for testings.
    // The plan is to write a Gtk+ gui later.
    if args.add != "".to_string() {
        let db = hammond::establish_connection();
        let _ = index_feed::insert_return_source(&db, &args.add);
    }

    if args.up {
        let db = hammond::establish_connection();
        index_feed::index_loop(db)?;
    }

    if args.dl >= 0 {
        let db = hammond::establish_connection();
        downloader::latest_dl(&db, args.dl as u32)?;
    }

    if args.latest {
        let db = hammond::establish_connection();
        let foo = dbqueries::get_episodes_with_limit(&db, 10)?;
        // This ends up horribly but works for now.
        let _: Vec<_> = foo.iter().map(|x| println!("{:?}", x)).collect();
    }

    Ok(())
}

quick_main!(run);
