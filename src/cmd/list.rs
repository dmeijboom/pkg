use anyhow::Result;
use chrono::{TimeZone, Utc};
use colored::Colorize;

use crate::store::{list_installed, Store};
use crate::utils::root_dir;

pub fn run() -> Result<()> {
    println!("{}", ">> fetching installed packages".blue());

    let store = Store::new(root_dir().join("store"));

    for tx in list_installed(&store)? {
        let time = Utc.timestamp(tx.created_at as i64, 0);
        let components = tx.package_id.split('@').collect::<Vec<_>>();

        println!(
            "- {} {}",
            components[0].green(),
            format!(
                "(version {} at {})",
                components[1].bold(),
                time.to_rfc3339().bold()
            )
            .white()
        );
    }

    Ok(())
}
