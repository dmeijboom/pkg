use anyhow::Result;
use clap::Parser;

mod cmd;
mod download;
mod install;
mod package;
mod pkgscript;
mod store;
mod utils;

#[derive(Parser)]
struct Args {
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Parser)]
enum Cmd {
    #[clap(about = "Install a package")]
    Install(cmd::install::Opts),
    #[clap(about = "Remove a package")]
    Remove(cmd::remove::Opts),
    #[clap(about = "List all installed packages")]
    List,
    #[clap(about = "Validate a package without installing it")]
    Check(cmd::check::Opts),
    #[clap(about = "Print shell completions")]
    Complete(cmd::complete::Opts),
    #[clap(about = "Manage repositories", subcommand)]
    Repo(RepoCmd),
}

#[derive(Parser)]
pub enum RepoCmd {
    #[clap(about = "List repositories")]
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Cmd::Install(opts) => cmd::install::run(opts).await,
        Cmd::Remove(opts) => cmd::remove::run(opts).await,
        Cmd::List => cmd::list::run().await,
        Cmd::Check(opts) => cmd::check::run(opts).await,
        Cmd::Complete(opts) => cmd::complete::run(opts),
        Cmd::Repo(cmd) => match cmd {
            RepoCmd::List => cmd::repo::list::run().await,
        },
    }
}
