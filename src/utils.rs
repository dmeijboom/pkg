use std::path::PathBuf;
use std::{env, fs};

use crate::package::Package;
use anyhow::Result;

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
