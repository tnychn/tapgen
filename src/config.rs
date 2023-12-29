use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) prefix: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let tilde = home::home_dir().unwrap();
        let prefix = tilde.join(".tapgen");
        Self {
            prefix: prefix.clone(),
        }
    }
}

impl Config {
    pub(crate) fn init() -> Result<Self> {
        let path = home::home_dir()
            .expect("failed to locate user home directory")
            .join(".tapgen.config.toml");

        let config = if !path.exists() {
            let config = Self::default();
            let contents = toml::to_string_pretty(&config)?;
            fs::write(path, contents)?;
            config
        } else {
            let contents = fs::read_to_string(path)?;
            toml::from_str(&contents)?
        };

        Ok(config)
    }
}
