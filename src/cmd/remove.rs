use anyhow::{anyhow, Result};
use clap::Parser;
use colored::Colorize;
use tokio::fs;

use crate::store::{is_installed, Action, Store, Transaction};
use crate::utils::{parse_id, root_dir};

#[derive(Parser)]
pub struct Opts {
    pub id: String,
}

pub async fn run(opts: Opts) -> Result<()> {
    let root = root_dir();
    let store = Store::new(root.join("store"));

    if !is_installed(&store, &opts.id).await? {
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

        let is_valid = match fs::read_link(entry.path()).await {
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

    let root_tx = store.root().await?;

    store
        .add(&Transaction::new(root_tx, opts.id, Action::Remove))
        .await?;

    Ok(())
}
