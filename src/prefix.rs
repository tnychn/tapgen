use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::OnceLock;

use anyhow::{bail, Error, Result};
use regex::Regex;

#[derive(Clone)]
pub(crate) struct Source(PathBuf);

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0.display(), f)
    }
}

impl FromStr for Source {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static PATTERN: OnceLock<Regex> = OnceLock::new();
        let pattern =
            PATTERN.get_or_init(|| Regex::new(r"^@:(?<path>[^\/]+(\/[^\/]+)*)$").unwrap());
        if let Some(captures) = pattern.captures(s) {
            let path = captures.name("path").unwrap().as_str();
            return Ok(Self(path.split('/').collect()));
        }
        bail!("mismatched prefix source pattern")
    }
}

impl AsRef<Path> for Source {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}
