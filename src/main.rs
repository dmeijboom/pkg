use anyhow::Result;
use clap::Parser;

use cmd::complete as complete_cmd;
use cmd::install as install_cmd;
use cmd::list as list_cmd;

mod cmd;
mod download;
mod installer;
mod package;
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
    #[clap(about = "Render completions")]
    Complete(complete_cmd::Opts),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Cmd::List => list_cmd::run(),
        Cmd::Install(opts) => install_cmd::run(opts),
        Cmd::Complete(opts) => complete_cmd::run(opts),
    }
}
