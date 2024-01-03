use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::OnceLock;

use anyhow::{bail, Context as _, Error, Result};
use regex::Regex;

use crate::{git, prompt};

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

impl std::fmt::Display for Host {
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
    host: Host,
    owner: String,
    repo: String,
    pub(crate) path: Option<PathBuf>,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "https://{}/{}/{}.git", self.host, self.owner, self.repo)
    }
}

impl FromStr for Source {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static PATTERN: OnceLock<Regex> = OnceLock::new();
        let pattern = PATTERN.get_or_init(|| {
            Regex::new(r"^(?<host>github|gitlab|bitbucket):(?<owner>[a-zA-Z0-9._-]+)\/(?<repo>[a-zA-Z0-9._-]+)(\/(?<path>[^\/]+(\/[^\/]+)*))?$").unwrap()
        });
        if let Some(captures) = pattern.captures(s) {
            return Ok(Self {
                host: Host::from_str(captures.name("host").unwrap().as_str()).unwrap(),
                owner: captures.name("owner").unwrap().as_str().to_string(),
                repo: captures.name("repo").unwrap().as_str().to_string(),
                path: captures
                    .name("path")
                    .map(|m| m.as_str().split('/').collect()),
            });
        }
        bail!("mismatched git source pattern")
    }
}

impl Source {
    pub(crate) fn resolve(&self, prefix: impl AsRef<Path>) -> Result<PathBuf> {
        if !git::check_installed()? {
            bail!("git is not installed; required for git source")
        }
        let mut dst = prefix.as_ref().join(&self.owner).join(&self.repo);
        if dst.exists() {
            println!("Repository already exists: '{}'", dst.display());
            println!("Checking for updates...");
            let repository = Repository::new(&dst);
            if repository
                .check_fastforwardable()
                .context("failed to check if git repository is fast-forwardable")?
            {
                if prompt::confirm("Outdated. Pull to update?", Some(true)) {
                    repository.pull()?;
                }
            } else {
                println!("Repository is up to date.");
            }
        } else {
            Repository::clone(self, &dst)?;
        }
        println!();
        if let Some(path) = &self.path {
            dst.push(path);
        }
        Ok(dst)
    }
}

pub(crate) struct Repository(PathBuf);

impl Repository {
    pub(crate) fn new(path: impl AsRef<Path>) -> Self {
        Self(path.as_ref().to_path_buf())
    }

    pub(crate) fn clone(src: impl ToString, dst: impl AsRef<Path>) -> Result<Self> {
        let status = Command::new("git")
            .arg("clone")
            .arg(src.to_string())
            .arg(dst.as_ref())
            .status()
            .context("failed to execute git clone command")?;
        if !status.success() {
            bail!("failed to clone git repository ({status})")
        }
        Ok(Self(dst.as_ref().to_path_buf()))
    }

    pub(crate) fn pull(&self) -> Result<()> {
        let status = Command::new("git")
            .arg("pull")
            .current_dir(&self.0)
            .status()
            .context("failed to execute git pull command")?;
        if !status.success() {
            bail!("failed to pull git repository ({status})")
        }
        Ok(())
    }

    pub(crate) fn check_fastforwardable(&self) -> Result<bool> {
        let status = Command::new("git")
            .arg("remote")
            .arg("update")
            .current_dir(&self.0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("failed to execute git remote update command")?;
        if !status.success() {
            bail!("failed to update remote refs of git repository ({status})")
        }
        let command = Command::new("git")
            .arg("status")
            .arg("-uno")
            .current_dir(&self.0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .context("failed to execute git status command")?;
        if !command.status.success() {
            bail!("failed to check updated status of git repository ({status})")
        }
        let output = String::from_utf8(command.stdout)
            .expect("command output encoding should be utf-8")
            .to_string();
        Ok(output.contains("can be fast-forwarded"))
    }
}

pub(crate) fn obtain_config() -> Result<HashMap<String, String>> {
    fn obtain_config_value(name: &str) -> Result<Option<String>> {
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

    let mut config = HashMap::new();
    if let Some(name) = obtain_config_value("user.name")? {
        config.insert(String::from("name"), name);
    }
    if let Some(email) = obtain_config_value("user.email")? {
        config.insert(String::from("email"), email);
    }
    Ok(config)
}

pub(crate) fn check_installed() -> Result<bool> {
    let check = Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to execute git version command")?;
    Ok(check.success())
}
