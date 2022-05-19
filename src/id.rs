use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use anyhow::{Error};
use serde::{Deserialize, Serialize};

use crate::utils::parse_id;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Id {
    pub name: String,
    pub version: String,
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

impl FromStr for Id {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, version) = parse_id(s)?;

        Ok(Id {
            name: name.to_string(),
            version: version.to_string(),
        })
    }
}
