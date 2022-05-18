mod content;
mod storage;
mod transaction;

use std::collections::HashMap;

pub use content::{Content, ContentType};
pub use storage::Storage;
pub use transaction::{Transaction, TransactionKind};

use anyhow::Result;

pub struct PackageMeta {
    pub content: Vec<Content>,
    pub package_id: String,
    pub created_at: u64,
}

pub struct RepositoryMeta {
    pub name: String,
    pub git_remote: String,
    pub created_at: u64,
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

    pub async fn list_installed(&self) -> Result<Vec<PackageMeta>> {
        let mut marked = HashMap::new();
        let mut packages = vec![];

        self.storage
            .walk(|tx| {
                match tx.kind {
                    TransactionKind::InstallPackage {
                        package_id,
                        content,
                    } if !marked.contains_key(&package_id) => {
                        marked.insert(package_id.clone(), true);
                        packages.push(PackageMeta {
                            package_id,
                            content,
                            created_at: tx.created_at,
                        });
                    }
                    TransactionKind::RemovePackage { package_id, .. }
                        if !marked.contains_key(&package_id) =>
                    {
                        marked.insert(package_id, false);
                    }
                    _ => {}
                };

                true
            })
            .await?;

        Ok(packages)
    }

    pub async fn find_added_repository(&self, repo_name: &str) -> Result<Option<Transaction>> {
        let mut repo = None;

        self.storage
            .walk(|tx| match &tx.kind {
                TransactionKind::AddRepository { name, .. } if name == repo_name => {
                    repo = Some(tx);
                    false
                }
                TransactionKind::RemoveRepository { name, .. } if name == repo_name => {
                    repo = None;
                    false
                }
                _ => true,
            })
            .await?;

        Ok(repo)
    }

    pub async fn list_repositories(&self) -> Result<Vec<RepositoryMeta>> {
        let mut marked = HashMap::new();
        let mut repositories = vec![];

        self.storage
            .walk(|tx| {
                match tx.kind {
                    TransactionKind::AddRepository {
                        name, git_remote, ..
                    } if !marked.contains_key(&name) => {
                        marked.insert(name.clone(), true);
                        repositories.push(RepositoryMeta {
                            name,
                            git_remote,
                            created_at: tx.created_at,
                        });
                    }
                    TransactionKind::RemoveRepository { name } if !marked.contains_key(&name) => {
                        marked.insert(name, false);
                    }
                    _ => {}
                }

                true
            })
            .await?;

        Ok(repositories)
    }

    pub async fn find_installed_package(
        &self,
        install_package_id: &str,
    ) -> Result<Option<Transaction>> {
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
