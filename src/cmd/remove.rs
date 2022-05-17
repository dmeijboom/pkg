use anyhow::{anyhow, Result};
use clap::Parser;
use colored::Colorize;
use tokio::fs;

use crate::store::{list_installed, Action, Store, Transaction};
use crate::utils::root_dir;

#[derive(Parser)]
pub struct Opts {
    pub id: String,
}

pub async fn run(opts: Opts) -> Result<()> {
    let root = root_dir();
    let store = Store::new(root.join("store"));
    let mut installed = list_installed(&store).await?;

    let idx = installed
        .iter()
        .position(|tx| tx.package_id == opts.id)
        .ok_or_else(|| anyhow!("packages not installed"))?;
    let install_tx = installed.remove(idx);

    println!("{}", format!(">> removing {}", opts.id).blue());

    for content in install_tx.content {
        if !content.published {
            continue;
        }

        let link = root.join("bin").join(content.filename);

        if link.exists() {
            fs::remove_file(link).await?;
        }
    }

    let mut read_dir = fs::read_dir(root.join("content")).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let is_dangling = !installed.iter().any(|tx| {
            tx.content
                .iter()
                .any(|c| Some(c.checksum.as_str()) == entry.file_name().to_str())
        });

        if !is_dangling {
            continue;
        }

        println!(
            "{}",
            format!(
                "removing dangling file: {}",
                entry.file_name().to_str().unwrap()
            )
            .white()
        );

        fs::remove_file(entry.path()).await?;
    }

    let root_tx = store.root().await?;

    store
        .add(&Transaction::new(root_tx, opts.id, Action::Remove))
        .await?;

    Ok(())
}
