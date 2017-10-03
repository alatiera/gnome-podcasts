#[macro_use]
extern crate log;
extern crate loggerv;

#[macro_use]
extern crate structopt_derive;
extern crate structopt;

#[macro_use]
extern crate error_chain;

extern crate hammond;

use structopt::StructOpt;
use hammond::errors::*;
use hammond::downloader;

#[derive(StructOpt, Debug)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Enable logging, use multiple `v`s to increase verbosity
    #[structopt(short = "v", long = "verbose")]
    verbosity: u64,
}

fn run() -> Result<()> {
    let args = Opt::from_args();

    loggerv::init_with_verbosity(args.verbosity)?;

    let foo = args;
    info!("{:?}", foo);

    hammond::init()?;
    let db = hammond::establish_connection();
    downloader::latest_dl(&db, 2)?;

    Ok(())
}

quick_main!(run);