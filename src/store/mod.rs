mod store;
mod transaction;

use std::collections::HashMap;
pub use store::Store;
pub use transaction::{Action, Transaction};

use anyhow::Result;

pub async fn list_installed(store: &Store) -> Result<Vec<Transaction>> {
    let mut transactions = vec![];

    store
        .walk(|tx| {
            transactions.push(tx);
            true
        })
        .await?;

    let mut index_map = HashMap::new();
    let mut installed = vec![];

    for tx in transactions.into_iter().rev() {
        match tx.action {
            Action::Install => {
                if index_map.contains_key(&tx.package_id) {
                    continue;
                }

                index_map.insert(tx.package_id.clone(), installed.len());
                installed.push(tx);
            }
            Action::Remove => {
                if let Some(index) = index_map.remove(&tx.package_id) {
                    installed.remove(index);
                }
            }
        };
    }

    Ok(installed)
}

pub async fn is_installed(store: &Store, package_id: &str) -> Result<bool> {
    let mut installed = false;

    store
        .walk(|tx| match tx.action {
            Action::Install if tx.package_id == package_id => {
                installed = true;
                false
            }
            Action::Remove if tx.package_id == package_id => {
                installed = true;
                false
            }
            _ => true,
        })
        .await?;

    Ok(installed)
}
