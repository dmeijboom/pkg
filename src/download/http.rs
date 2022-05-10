use anyhow::Result;
use std::io::Read;
use url::Url;

pub fn download(url: Url) -> Result<Box<dyn Read>> {
    let response = reqwest::blocking::get(url)?.error_for_status()?;

    Ok(Box::new(response))
}
