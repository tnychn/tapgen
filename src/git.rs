use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::OnceLock;

use anyhow::{bail, Context, Error, Ok, Result};
use regex::Regex;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Config {
    pub(crate) name: Option<String>,
    pub(crate) email: Option<String>,
}

impl Config {
    fn obtain_value(name: &str) -> Result<Option<String>> {
        let command = Command::new("git")
            .arg("config")
            .arg("--global")
            .arg(name)
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .context("failed to execute git config command")?;

        let value = if command.status.success() {
            let output = String::from_utf8(command.stdout)
                .expect("command output encoding should be utf-8")
                .trim()
                .to_string();
            Some(output)
        } else {
            None
        };

        Ok(value)
    }

    pub(crate) fn obtain() -> Result<Self> {
        if !check_git_installed()? {
            return Ok(Self {
                name: None,
                email: None,
            });
        }
        Ok(Self {
            name: Self::obtain_value("user.name")?,
            email: Self::obtain_value("user.email")?,
        })
    }
}

#[derive(Clone)]
pub(crate) enum Host {
    GitHub,
    GitLab,
    BitBucket,
}

impl FromStr for Host {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "github" => Ok(Self::GitHub),
            "gitlab" => Ok(Self::GitLab),
            "bitbucket" => Ok(Self::BitBucket),
            _ => bail!("unidentified git host: '{s}'"),
        }
    }
}

impl Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitHub => write!(f, "github.com"),
            Self::GitLab => write!(f, "gitlab.com"),
            Self::BitBucket => write!(f, "bitbucket.org"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Source {
    pub(crate) host: Host,
    pub(crate) owner: String,
    pub(crate) repo: String,
}

impl Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "https://{}/{}/{}.git", self.host, self.owner, self.repo)
    }
}

impl FromStr for Source {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static PATTERN: OnceLock<Regex> = OnceLock::new();
        let pattern = PATTERN.get_or_init(|| {
            Regex::new(r"^(github|gitlab|bitbucket):([a-zA-Z0-9._-]+)\/([a-zA-Z0-9._-]+)$").unwrap()
        });
        if let Some(captures) = pattern.captures(s) {
            let (_, [host, owner, repo]) = captures.extract();
            return Ok(Self {
                host: Host::from_str(host).unwrap(),
                owner: owner.to_string(),
                repo: repo.to_string(),
            });
        }
        bail!("mismatched git source pattern")
    }
}

impl Source {
    pub(crate) fn clone(&self, dst: impl AsRef<Path>) -> Result<PathBuf> {
        let dst = dst.as_ref().join(&self.owner).join(&self.repo);

        if dst.exists() {
            return Ok(dst);
        }

        if !check_git_installed()? {
            bail!("git is not installed")
        }

        let status = Command::new("git")
            .arg("clone")
            .arg(self.to_string())
            .arg(&dst)
            .status()
            .context("failed to execute git clone command")?;
        println!();

        if !status.success() {
            bail!("failed to clone git repository")
        }

        Ok(dst)
    }
}

fn check_git_installed() -> Result<bool> {
    let check = Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to check if git is installed")?;
    Ok(check.success())
}
