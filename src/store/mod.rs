mod store;
mod transaction;

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

    Ok(transactions
        .into_iter()
        .filter(|tx| tx.action == Action::Install)
        .rev()
        .collect())
}
