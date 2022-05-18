use anyhow::{anyhow, Result};
use chrono::{TimeZone, Utc};
use clap::Parser;
use colored::Colorize;
use git2::{Buf, Repository};
use temp_dir::TempDir;
use tokio::fs;

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
        let storage = Storage::new(root.join("store"));
        let mut store = Store::new(&storage);

        if store.find_added_repository(&opts.name).await?.is_some() {
            return Err(anyhow!("repository already added"));
        }

        let git_remote = format!("https://github.com/{}.git", opts.name);

        println!("{}", format!("pulling {}", git_remote).white());

        let repos_dir = root.join("repos");

        if !repos_dir.exists() {
            fs::create_dir_all(&repos_dir).await?;
        }

        let tmp_dir = TempDir::new()?;
        let repo = Repository::clone(&git_remote, tmp_dir.path())?;
        let commit_id = repo.head()?.peel_to_commit()?.id();

        let mut builder = repo.packbuilder()?;
        let mut buf = Buf::new();

        builder.insert_commit(commit_id)?;
        builder.write_buf(&mut buf)?;

        fs::write(repos_dir.join(opts.name.replace('/', "_")), &*buf).await?;

        store
            .add(Transaction::new(TransactionKind::AddRepository {
                name: opts.name,
                git_remote,
                version: commit_id.to_string()[..7].to_string(),
            }))
            .await?;

        println!("{}", "✓ repository added".green());

        Ok(())
    }
}
