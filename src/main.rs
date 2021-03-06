use anyhow::Result;
use clap::Parser;

mod cmd;
mod download;
mod id;
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
    Add(cmd::add::Opts),
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
    #[clap(about = "Add a repository")]
    Add(cmd::repo::add::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Cmd::Add(opts) => cmd::add::run(opts).await,
        Cmd::Remove(opts) => cmd::remove::run(opts).await,
        Cmd::List => cmd::list::run().await,
        Cmd::Check(opts) => cmd::check::run(opts).await,
        Cmd::Complete(opts) => cmd::complete::run(opts),
        Cmd::Repo(cmd) => match cmd {
            RepoCmd::Add(opts) => cmd::repo::add::run(opts).await,
            RepoCmd::List => cmd::repo::list::run().await,
        },
    }
}
