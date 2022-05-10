use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_dhall::StaticType;

#[derive(Serialize, Deserialize, StaticType)]
pub enum Action {
    Install,
}

#[derive(Serialize, Deserialize, StaticType)]
pub struct Transaction {
    pub before: Option<String>,
    pub created_at: u64,
    pub package_id: String,
    pub action: Action,
}

impl Transaction {
    pub fn new(before: Option<String>, package_id: String, action: Action) -> Self {
        Self {
            before,
            package_id,
            action,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}
