mod content;
mod storage;
mod transaction;

use std::collections::HashMap;

use anyhow::Result;

use crate::id::Id;
use crate::package::Package;

pub use content::{Content, ContentType};
pub use storage::Storage;
pub use transaction::{Transaction, TransactionKind};

pub struct PackageMeta {
    pub content: Vec<Content>,
    pub name: String,
    pub version: String,
    pub created_at: u64,
}

pub struct RepositoryMeta {
    pub name: String,
    pub git_remote: String,
    pub packages: Vec<Package>,
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
                            name: package_id.name,
                            version: package_id.version,
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

    pub async fn find_added_repository(&self, repo_name: &str) -> Result<Option<RepositoryMeta>> {
        let mut repo = None;

        self.storage
            .walk(|tx| match tx.kind {
                TransactionKind::AddRepository {
                    name,
                    git_remote,
                    packages,
                    ..
                } if name == repo_name => {
                    repo = Some(RepositoryMeta {
                        name,
                        git_remote,
                        packages,
                        created_at: tx.created_at,
                    });
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
                        name,
                        git_remote,
                        packages,
                        ..
                    } if !marked.contains_key(&name) => {
                        marked.insert(name.clone(), true);
                        repositories.push(RepositoryMeta {
                            name,
                            git_remote,
                            packages,
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
        install_package_id: &Id,
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

    pub async fn search_package(&self, package_id: &Id) -> Result<Option<Package>> {
        Ok(self
            .list_repositories()
            .await?
            .into_iter()
            .flat_map(|repo| repo.packages)
            .find(|package| {
                package.name == package_id.name && package.version == package_id.version
            }))
    }
}
