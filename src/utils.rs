use std::path::PathBuf;
use std::{env, fs};

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};

use crate::package::Package;

pub fn root_dir() -> PathBuf {
    PathBuf::from(format!(
        "{}/.pkg",
        env::var("HOME").expect("HOME directory not set")
    ))
}

pub fn parse_package_config(filename: PathBuf) -> Result<Package> {
    let content = fs::read_to_string(filename)?;
    let package = serde_dhall::from_str(&content).imports(true).parse()?;

    Ok(package)
}

pub fn parse_id(id: &str) -> Result<(&str, &str)> {
    let components = id.split('@').collect::<Vec<_>>();

    if components.len() != 2 {
        return Err(anyhow!("invalid package id format"));
    }

    Ok((components[0], components[1]))
}

pub fn sha256sum(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_ref());

    hex::encode(hasher.finalize())
}
