use std::time::{SystemTime, UNIX_EPOCH};

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::store::content::Content;

#[derive(Debug, Serialize, Deserialize)]
pub enum TransactionKind {
    InstallPackage {
        package_id: String,
        content: Vec<Content>,
    },
    RemovePackage {
        package_id: String,
    },
    AddRepository {
        name: String,
        git_remote: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub struct Transaction {
    #[bincode(with_serde)]
    pub kind: TransactionKind,
    pub before: Option<String>,
    pub created_at: u64,
}

impl Transaction {
    pub fn new(kind: TransactionKind) -> Self {
        Self {
            kind,
            before: None,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn with_before(mut self, before: String) -> Self {
        self.before = Some(before);
        self
    }
}
