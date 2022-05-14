use anyhow::Result;
use futures::io::{Error, ErrorKind};
use futures::stream::TryStreamExt;
use tokio::io::AsyncBufRead;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use url::Url;

pub async fn download(url: Url) -> Result<Box<dyn AsyncBufRead + Sync + Send + Unpin>> {
    let response = reqwest::get(url).await?.error_for_status()?;
    let stream = response.bytes_stream();
    let read = stream
        .map_err(|e| Error::new(ErrorKind::Other, e))
        .into_async_read();

    Ok(Box::new(read.compat()))
}
