use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_dhall::StaticType;

use crate::store::content::Content;

#[derive(Debug, Serialize, Deserialize, PartialEq, StaticType)]
pub enum Action {
    Install,
    Remove,
}

#[derive(Debug, Serialize, Deserialize, StaticType)]
pub struct Transaction {
    pub before: Option<String>,
    pub created_at: u64,
    pub package_id: String,
    pub action: Action,
    pub content: Vec<Content>,
}

impl Transaction {
    pub fn new(before: Option<String>, package_id: String, action: Action) -> Self {
        Self {
            before,
            package_id,
            action,
            content: vec![],
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn with_content(mut self, content: Vec<Content>) -> Self {
        self.content = content;
        self
    }
}
