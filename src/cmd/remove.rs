use anyhow::{anyhow, Result};
use clap::Parser;
use colored::Colorize;
use tokio::fs;

use crate::store::{Storage, Store, Transaction, TransactionKind};
use crate::utils::root_dir;

#[derive(Parser)]
pub struct Opts {
    pub id: String,
}

pub async fn run(opts: Opts) -> Result<()> {
    let root = root_dir();
    let storage = Storage::new(root.join("store"));
    let mut store = Store::new(&storage);
    let mut installed = store.list_installed().await?;

    let idx = installed
        .iter()
        .position(|tx| tx.package_id == opts.id)
        .ok_or_else(|| anyhow!("packages not installed"))?;
    let content = installed.remove(idx).content;

    println!("{}", format!(">> removing {}", opts.id).blue());

    for content in content.iter() {
        if !content.published {
            continue;
        }

        let link = root.join("bin").join(&content.filename);

        if link.exists() {
            fs::remove_file(link).await?;
        }
    }

    let mut read_dir = fs::read_dir(root.join("content")).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let is_dangling = !installed.iter().any(|meta| {
            meta.content
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

    store
        .add(Transaction::new(TransactionKind::RemovePackage {
            package_id: opts.id,
        }))
        .await?;

    println!("{}", "✓ package removed".green());

    Ok(())
}
