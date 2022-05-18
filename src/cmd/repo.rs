use anyhow::{anyhow, Result};
use chrono::{TimeZone, Utc};
use clap::Parser;
use colored::Colorize;
use tokio::fs;
use git2::Repository;

use crate::store::{Storage, Store, Transaction, TransactionKind};
use crate::utils::root_dir;

pub mod list {
    use super::*;

    pub async fn run() -> Result<()> {
        println!("{}", ">> fetching repositories".blue());

        let storage = Storage::new(root_dir().join("store"));
        let store = Store::new(&storage);

        for meta in store.list_repositories().await? {
            let time = Utc.timestamp(meta.created_at as i64, 0);

            println!(
                "{} {}",
                meta.name.green(),
                format!("(at {})", time.to_rfc3339().bold()).white()
            );
        }

        Ok(())
    }
}

pub mod add {
    use super::*;

    #[derive(Parser)]
    pub struct Opts {
        pub name: String,
    }

    pub async fn run(opts: Opts) -> Result<()> {
        println!("{}", format!(">> adding repository {}", opts.name).blue());

        let root = root_dir();
        let storage = Storage::new(root);
        let store = Store::new(&storage);

        if store.find_added_repository(&opts.name).await?.is_some() {
            return Err(anyhow!("repository already added"));
        }

        let git_remote = format!("https://github.com/{}.git", opts.name);

        println!("{}", format!("pulling {}", git_remote).white());

        let id = opts.name.replace("/", "-");
        let repos_dir = root.join("repos");

        if !repos_dir.exists() {
            fs::create_dir_all(&repos_dir).await?;
        }

        let dest = repos_dir.join(id);
        let repo = Repository::clone(git_rem, dest)?;

        store.add(Transaction::new(TransactionKind::AddRepository {
            name: opts.name,
            git_remote,
            version: repo.head()?.
        }))?;

        Ok(())
    }
}
