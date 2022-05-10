use anyhow::Result;
use chrono::{TimeZone, Utc};
use clap::Parser;
use colored::Colorize;

use crate::store::{Action, Store};
use crate::utils::root_dir;

#[derive(Parser, Debug)]
pub struct Opts {}

pub fn run(_opts: Opts) -> Result<()> {
    println!("{}", ">> fetching installed packages".blue());

    let store = Store::new(root_dir().join("store"));
    let mut transactions = vec![];

    if let Some(root_hash) = store.root()? {
        let mut before = Some(root_hash);

        while let Some(hash) = before.take() {
            let tx = store.read(&hash)?;

            before = tx.before.clone();
            transactions.push(tx);
        }
    }

    for tx in transactions
        .iter()
        .filter(|tx| tx.action == Action::Install)
        .rev()
    {
        let time = Utc.timestamp(tx.created_at as i64, 0);
        let components = tx.package_id.split("@").collect::<Vec<_>>();

        println!("- {} {}", components[0].green(), format!("(version {} at {})", components[1].bold(), time.to_rfc3339().bold()).white());
    }

    Ok(())
}
