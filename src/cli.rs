use structopt::StructOpt;
use loggerv;
use errors::*;
// use models::NewPodcast;

#[derive(StructOpt, Debug)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Enable logging, use multiple `v`s to increase verbosity
    #[structopt(short = "v", long = "verbose")]
    verbosity: u64,
}

pub fn run() -> Result<()> {
    let args = Opt::from_args();

    loggerv::init_with_verbosity(args.verbosity)?;

    let foo = args;
    info!("{:?}", foo);

    ::init()?;
    ::index_feed::foo();

    Ok(())
}