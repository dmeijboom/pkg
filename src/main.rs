use anyhow::Result;
use clap::Parser;

use cmd::install as install_cmd;
use cmd::list as list_cmd;

mod cmd;
mod config;
mod download;
mod pkgscript;
mod store;
mod utils;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Parser, Debug)]
enum Cmd {
    #[clap(about = "Install a package")]
    Install(install_cmd::Opts),
    #[clap(about = "List all installed packages")]
    List,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Cmd::Install(opts) => install_cmd::run(opts),
        Cmd::List => list_cmd::run(),
    }
}
