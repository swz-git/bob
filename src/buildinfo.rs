use std::{default, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod hex_ser {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:016x}", value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u64::from_str_radix(&s, 16).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    pub projects: Vec<Project>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    #[serde(with = "hex_ser")]
    pub hash: u64,
    #[serde(with = "toml_datetime_compat")]
    pub build_date: DateTime<Utc>,
}

impl FromStr for BuildInfo {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(toml::from_str(s)?)
    }
}
impl ToString for BuildInfo {
    fn to_string(&self) -> String {
        toml::to_string_pretty(self).unwrap() // this should never fail i think
    }
}

impl BuildInfo {
    pub fn new() -> Self {
        BuildInfo {
            projects: Default::default(),
        }
    }
}
