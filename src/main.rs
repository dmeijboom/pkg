use anyhow::Result;
use clap::Parser;

use cmd::install as install_cmd;

mod cmd;
mod config;
mod download;
mod pkgscript;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Parser, Debug)]
enum Cmd {
    #[clap(about = "Install a package")]
    Install(install_cmd::Opts),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Cmd::Install(opts) => install_cmd::run(opts),
    }
}
