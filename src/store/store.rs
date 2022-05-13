use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::store::Transaction;
use sha2::{Digest, Sha256};

pub struct Store {
    root_dir: PathBuf,
}

impl Store {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn root(&self) -> Result<Option<String>> {
        let file = self.root_dir.join("root");

        if !file.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(file)?;

        Ok(Some(content))
    }

    pub fn read(&self, hash: &str) -> Result<Transaction> {
        let file = self.root_dir.join(hash);

        if !file.exists() {
            return Err(anyhow!("transaction '{}' does not exist", hash));
        }

        let content = fs::read_to_string(file)?;
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());

        let expected = hex::encode(hasher.finalize());

        if hash != expected {
            return Err(anyhow!(
                "checksum mismatch (expected: '{}', got: '{}')",
                expected,
                hash
            ));
        }

        let tx = serde_dhall::from_str(&content).parse()?;

        Ok(tx)
    }

    pub fn add(&self, tx: &Transaction) -> Result<String> {
        let output = serde_dhall::serialize(tx)
            .static_type_annotation()
            .to_string()?;
        let mut hasher = Sha256::new();
        hasher.update(output.as_bytes());

        let hash = hex::encode(hasher.finalize());

        if !self.root_dir.exists() {
            fs::create_dir_all(&self.root_dir)?;
        }

        fs::write(self.root_dir.join(&hash), output)?;
        fs::write(self.root_dir.join("root"), hash.clone())?;

        Ok(hash)
    }
}
