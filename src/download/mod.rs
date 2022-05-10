use std::fs::File;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{anyhow, Result};
use url::Url;

mod http;

pub fn download(source: impl AsRef<str>, dest: impl AsRef<Path>) -> Result<()> {
    let uri = Url::parse(source.as_ref())?;
    let path = PathBuf::from(uri.path());
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow!("filename missing"))?;
    let path = PathBuf::from(dest.as_ref()).join(filename);

    let mut file = match uri.scheme() {
        "https" => http::download(uri),
        "http" => Err(anyhow!("'http' scheme is unsafe and unsupported")),
        _ => Err(anyhow!("unsupported scheme '{}'", uri.scheme())),
    }?;

    if !dest.as_ref().exists() {
        fs::create_dir_all(dest.as_ref())?;
    }

    let mut out = File::create(path)?;

    io::copy(&mut file, &mut out)?;

    Ok(())
}
