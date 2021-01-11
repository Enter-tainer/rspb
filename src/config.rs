use std::usize;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub port: u16,
    pub max_length: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 3999,
            max_length: 5_000_000,
        }
    }
}

impl Config {
    pub async fn load(path: Option<&str>) -> Option<Self> {
        let file = tokio::fs::read_to_string(path.unwrap_or("config.yaml")).await;
        if let Ok(str) = file {
            return Some(serde_yaml::from_str(&str).unwrap());
        }
        None
    }
}
