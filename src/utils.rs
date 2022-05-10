use std::env;
use std::path::PathBuf;

pub fn root_dir() -> PathBuf {
    PathBuf::from(format!(
        "{}/.pkg",
        env::var("HOME").expect("HOME directory not set")
    ))
}
