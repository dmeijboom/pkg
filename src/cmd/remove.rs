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

    let tx = is_installed(&store, &opts.id)
        .await?
        .ok_or_else(|| anyhow!("package is not installed"))?;

    println!("{}", format!(">> removing {}", opts.id).blue());

    for content in tx.content {
        
    }

    let root_tx = store.root().await?;

    store
        .add(&Transaction::new(root_tx, opts.id, Action::Remove))
        .await?;

    Ok(())
}
