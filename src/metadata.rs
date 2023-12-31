use std::path::{Path, PathBuf};

use std::sync::OnceLock;

use glob::{Pattern, PatternError};
use regex::Regex;
use serde::Deserialize;

use crate::utils::Result;

#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct Url(String);

impl TryFrom<String> for Url {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        static URL_PATTERN: OnceLock<Regex> = OnceLock::new();
        if !URL_PATTERN.get_or_init(||Regex::new(r"https?:\/\/(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)").unwrap()).is_match(&value) {
            return Err(format!("invalid url: '{value}'"));
        }
        Ok(Self(value))
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(try_from = "Vec<String>")]
pub struct GlobPatterns(Vec<Pattern>);

impl TryFrom<Vec<String>> for GlobPatterns {
    type Error = PatternError;

    fn try_from(patterns: Vec<String>) -> Result<Self, Self::Error> {
        Ok(GlobPatterns(
            patterns
                .iter()
                .map(|pattern| Pattern::new(pattern))
                .collect::<Result<Vec<Pattern>, PatternError>>()?,
        ))
    }
}

impl GlobPatterns {
    pub fn matches_path_any<P: AsRef<Path>>(&self, path: P) -> bool {
        self.0.iter().any(|p| p.matches_path(path.as_ref()))
    }

    pub(crate) fn push(&mut self, value: Pattern) {
        self.0.push(value)
    }
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
    #[serde(rename = "__name__")]
    pub name: String,
    #[serde(rename = "__author__")]
    pub author: String,
    #[serde(rename = "__url__")]
    pub url: Option<Url>,
    #[serde(rename = "__description__")]
    pub description: Option<String>,
    #[serde(rename = "__base__", default)]
    pub base: PathBuf, // relative path
    #[serde(rename = "__copy__", default)]
    pub copy: GlobPatterns,
    #[serde(rename = "__exclude__", default)]
    pub exclude: GlobPatterns,
}
