use anyhow::{anyhow, Result};
use clap::Parser;
use colored::Colorize;
use tokio::fs;

use crate::store::{list_installed, Action, Store, Transaction};
use crate::utils::{parse_id, root_dir};

#[derive(Debug, Parser)]
pub struct Opts {
    pub id: String,
}

pub async fn run(opts: Opts) -> Result<()> {
    let root = root_dir();
    let store = Store::new(root.join("store"));

    if !list_installed(&store)?
        .iter()
        .any(|tx| tx.package_id == opts.id)
    {
        return Err(anyhow!("package is not installed"));
    }

    println!("{}", format!(">> removing {}", opts.id).blue());

    let (name, version) = parse_id(&opts.id)?;
    let pkg_dir = root.join("packages").join(name).join(version);

    fs::remove_dir_all(&pkg_dir).await?;

    let mut read_dir = fs::read_dir(root.join("bin")).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        if !entry.path().is_symlink() {
            continue;
        }

        let is_valid = match entry.path().read_link() {
            Ok(target) => target.exists(),
            Err(_) => false,
        };

        if !is_valid {
            fs::remove_file(entry.path()).await?;

            println!(
                "{}",
                format!(
                    "unpublishing {}",
                    entry.path().file_name().and_then(|f| f.to_str()).unwrap()
                )
                .white()
            );
        }
    }

    store.add(&Transaction::new(store.root()?, opts.id, Action::Remove))?;

    Ok(())
}
