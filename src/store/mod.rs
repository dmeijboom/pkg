mod store;
mod transaction;

use std::collections::HashMap;
pub use store::Store;
pub use transaction::{Action, Transaction};

use anyhow::Result;

pub fn list_installed(store: &Store) -> Result<Vec<Transaction>> {
    let mut transactions = vec![];

    if let Some(root_hash) = store.root()? {
        let mut before = Some(root_hash);

        while let Some(hash) = before.take() {
            let tx = store.read(&hash)?;

            before = tx.before.clone();
            transactions.push(tx);
        }
    }

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
