use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ContentType {
    Executable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub published: bool,
    pub checksum: String,
    pub filename: String,
    pub content_type: ContentType,
}

impl Content {
    pub fn new(content_type: ContentType, filename: String, checksum: String) -> Self {
        Self {
            published: false,
            checksum,
            filename,
            content_type,
        }
    }
}
