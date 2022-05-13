use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;
use url::Url;

mod http;

pub struct ChecksumReader<R: Read> {
    reader: R,
    hasher: Sha256,
}

impl<R: Read> Read for ChecksumReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(buf)?;

        if n > 0 {
            self.hasher.update(&buf[..n]);
        }

        Ok(n)
    }
}

impl<R: Read> ChecksumReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            hasher: Sha256::new(),
        }
    }

    pub fn compute(self) -> Result<String> {
        let output = hex::encode(self.hasher.finalize());

        Ok(output)
    }
}

pub fn download_and_unpack(source: &str, dest: impl AsRef<Path>) -> Result<String> {
    let uri = Url::parse(source)?;
    let path = PathBuf::from(uri.path());
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow!("filename missing"))?;
    let file = match uri.scheme() {
        "https" => http::download(uri),
        "http" => Err(anyhow!("'http' scheme is unsafe and unsupported")),
        _ => Err(anyhow!("unsupported scheme '{}'", uri.scheme())),
    }?;
    let mut file = ChecksumReader::new(file);

    if !dest.as_ref().exists() {
        fs::create_dir_all(dest.as_ref())?;
    }

    if filename.to_str().unwrap().ends_with(".tar.gz") {
        let tar = GzDecoder::new(file);
        let mut archive = Archive::new(tar);

        archive.unpack(dest.as_ref())?;

        return archive.into_inner().into_inner().compute();
    }

    let dest_path = PathBuf::from(dest.as_ref()).join(filename);
    let mut out = File::create(dest_path)?;

    io::copy(&mut file, &mut out)?;

    file.compute()
}
