use std::path::PathBuf;

use anyhow::{anyhow, Result};
use bincode::config;
use tokio::fs;

use crate::store::Transaction;
use crate::utils::sha256sum;

pub struct Storage {
    root_dir: PathBuf,
}

impl Storage {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub async fn walk(&self, mut f: impl FnMut(Transaction) -> bool) -> Result<()> {
        if let Some(hash) = self.root().await? {
            let mut tx = self.read(&hash).await?;

            loop {
                let mut next = None;

                if let Some(hash) = &tx.before {
                    next = Some(self.read(hash).await?);
                }

                if !f(tx) {
                    return Ok(());
                }

                match next {
                    Some(next) => tx = next,
                    None => break,
                }
            }
        }

        Ok(())
    }

    pub async fn root(&self) -> Result<Option<String>> {
        let file = self.root_dir.join("root");

        if !file.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(file).await?;

        Ok(Some(content))
    }

    pub async fn read(&self, hash: &str) -> Result<Transaction> {
        let file = self.root_dir.join(hash);

        if !file.exists() {
            return Err(anyhow!("transaction '{}' does not exist", hash));
        }

        let content = fs::read(file).await?;
        let expected = sha256sum(&content);

        if hash != expected {
            return Err(anyhow!(
                "checksum mismatch (expected: '{}', got: '{}')",
                expected,
                hash
            ));
        }

        let (tx, _) = bincode::decode_from_slice(&content, config::standard())?;

        Ok(tx)
    }

    pub async fn add(&self, tx: &Transaction) -> Result<String> {
        let output = bincode::encode_to_vec(tx, config::standard())?;
        let hash = sha256sum(&output);

        if !self.root_dir.exists() {
            fs::create_dir_all(&self.root_dir).await?;
        }

        fs::write(self.root_dir.join(&hash), output).await?;
        fs::write(self.root_dir.join("root"), hash.clone()).await?;

        Ok(hash)
    }
}
