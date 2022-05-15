use std::fs;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::{anyhow, Result};
use async_compression::tokio::bufread::{GzipDecoder, XzDecoder};
use sha2::digest::Update;
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio::io;
use tokio::io::{AsyncBufRead, AsyncRead, ReadBuf};
use tokio_tar::Archive;
use url::Url;

mod http;

macro_rules! unwrap_archive {
    ($archive:expr) => {
        $archive
            .into_inner()
            .map_err(|_| anyhow!("unable to unwrap inner reader"))?
            .into_inner()
            .compute()
    };
}

#[derive(Debug, PartialEq)]
enum CompressionFormat {
    TarGz,
    TarXz,
}

fn parse_compression_format(filename: &str) -> Option<CompressionFormat> {
    if filename.ends_with(".tar.gz") {
        return Some(CompressionFormat::TarGz);
    }

    if filename.ends_with(".tar.xz") {
        return Some(CompressionFormat::TarXz);
    }

    None
}

pub struct ChecksumReader<R: AsyncBufRead + Send + Sync + Unpin> {
    reader: R,
    hasher: Sha256,
}

impl<R: AsyncBufRead + Send + Sync + Unpin> AsyncRead for ChecksumReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let checksum_reader = Pin::into_inner(self);

        match Pin::new(&mut checksum_reader.reader).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                Update::update(&mut checksum_reader.hasher, buf.filled());

                Poll::Ready(Ok(()))
            }
            poll => poll,
        }
    }
}

impl<R: AsyncBufRead + Send + Sync + Unpin> AsyncBufRead for ChecksumReader<R> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
        let checksum_reader = Pin::into_inner(self);

        Pin::new(&mut checksum_reader.reader).poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        let checksum_reader = Pin::into_inner(self);

        Pin::new(&mut checksum_reader.reader).consume(amt)
    }
}
impl<R: AsyncBufRead + Send + Sync + Unpin> ChecksumReader<R> {
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

pub async fn download_and_unpack(source: &str, dest: impl AsRef<Path>) -> Result<String> {
    let uri = Url::parse(source)?;
    let path = PathBuf::from(uri.path());
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow!("filename missing"))?;
    let file = match uri.scheme() {
        "https" => http::download(uri).await?,
        "http" => return Err(anyhow!("'http' scheme is unsafe and unsupported")),
        _ => return Err(anyhow!("unsupported scheme '{}'", uri.scheme())),
    };
    let mut file = ChecksumReader::new(file);

    if !dest.as_ref().exists() {
        fs::create_dir_all(dest.as_ref())?;
    }

    let format = parse_compression_format(filename.to_str().unwrap());

    match format {
        Some(CompressionFormat::TarGz) => {
            let mut archive = Archive::new(GzipDecoder::new(file));
            archive.unpack(dest.as_ref()).await?;

            unwrap_archive!(archive)
        }
        Some(CompressionFormat::TarXz) => {
            let mut archive = Archive::new(XzDecoder::new(file));
            archive.unpack(dest.as_ref()).await?;

            unwrap_archive!(archive)
        }
        _ => {
            let dest_path = PathBuf::from(dest.as_ref()).join(filename);
            let mut out = File::create(dest_path).await?;

            io::copy(&mut file, &mut out).await?;

            file.compute()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_compression_format() {
        assert_eq!(
            parse_compression_format("foo.tar.gz"),
            Some(CompressionFormat::TarGz)
        );
        assert_eq!(
            parse_compression_format("foo.tar.xz"),
            Some(CompressionFormat::TarXz)
        );

        assert_eq!(parse_compression_format("foo"), None);
    }
}
