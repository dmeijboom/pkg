use anyhow::Result;
use chrono::{TimeZone, Utc};
use colored::Colorize;

use crate::store::{Storage, Store};
use crate::utils::root_dir;

pub async fn run() -> Result<()> {
    println!("{}", ">> fetching installed packages".blue());

    let storage = Storage::new(root_dir().join("store"));
    let store = Store::new(&storage);

    for meta in store.list_installed().await? {
        let time = Utc.timestamp(meta.created_at as i64, 0);

        println!(
            "{} {}",
            meta.name.green(),
            format!(
                "(version {} at {})",
                meta.version.bold(),
                time.to_rfc3339().bold()
            )
            .white()
        );
    }

    Ok(())
}
