use serde::Deserialize;
use serde_dhall::StaticType;

#[derive(Deserialize, Debug, StaticType)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: Vec<String>,
    pub install: String,
}
