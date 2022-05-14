use anyhow::Result;
use clap::Parser;

use cmd::{check as check_cmd, complete as complete_cmd, install as install_cmd, list as list_cmd};

mod cmd;
mod download;
mod install;
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
    #[clap(about = "Validate a package without installing it")]
    Check(check_cmd::Opts),
    #[clap(about = "Print shell completions")]
    Complete(complete_cmd::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Cmd::Install(opts) => install_cmd::run(opts).await,
        Cmd::List => list_cmd::run(),
        Cmd::Check(opts) => check_cmd::run(opts).await,
        Cmd::Complete(opts) => complete_cmd::run(opts),
    }
}
