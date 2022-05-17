mod content;
mod storage;
mod transaction;

use std::collections::HashMap;

pub use content::{Content, ContentType};
pub use storage::Storage;
pub use transaction::{Transaction, TransactionKind};

use anyhow::Result;

pub struct InstallMeta {
    pub content: Vec<Content>,
    pub package_id: String,
    pub installed_at: u64,
}

pub struct Store<'s> {
    storage: &'s Storage,
}

impl<'s> Store<'s> {
    pub fn new(storage: &'s Storage) -> Self {
        Self { storage }
    }

    pub async fn add(&mut self, mut tx: Transaction) -> Result<()> {
        if let Some(hash) = self.storage.root().await? {
            tx = tx.with_before(hash);
        }

        self.storage.add(&tx).await?;

        Ok(())
    }

    pub async fn list_installed(&self) -> Result<Vec<InstallMeta>> {
        let mut transactions = vec![];

        self.storage
            .walk(|tx| {
                transactions.push(tx);
                true
            })
            .await?;

        let mut index_map = HashMap::new();
        let mut installed = vec![];

        for tx in transactions.into_iter().rev() {
            match tx.kind {
                TransactionKind::InstallPackage {
                    package_id,
                    content,
                } => {
                    if index_map.contains_key(&package_id) {
                        continue;
                    }

                    index_map.insert(package_id.clone(), installed.len());
                    installed.push(InstallMeta {
                        package_id,
                        content,
                        installed_at: tx.created_at,
                    });
                }
                TransactionKind::RemovePackage { package_id, .. } => {
                    if let Some(index) = index_map.remove(&package_id) {
                        installed.remove(index);
                    }
                }
                _ => {}
            };
        }

        Ok(installed)
    }

    pub async fn is_installed(&self, install_package_id: &str) -> Result<Option<Transaction>> {
        let mut installed = None;

        self.storage
            .walk(|tx| match &tx.kind {
                TransactionKind::InstallPackage { package_id, .. }
                    if package_id == install_package_id =>
                {
                    installed = Some(tx);
                    false
                }
                TransactionKind::RemovePackage { package_id, .. }
                    if package_id == install_package_id =>
                {
                    installed = None;
                    false
                }
                _ => true,
            })
            .await?;

        Ok(installed)
    }
}
