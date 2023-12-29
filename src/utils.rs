use std::fs;
use std::path::Path;

use memchr::memchr;

// TODO: improve error structure and propagation
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    MiniJinja(#[from] minijinja::Error),
    #[error(transparent)]
    DeserializeToml(#[from] toml::de::Error),
    #[error("invalid variable: '{name}'")]
    ValidateVariable {
        name: String,
        source: InvalidVariableError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum InvalidVariableError {
    #[error("pattern with choices")]
    PatternWithChoices,
    #[error("unreasonable range")]
    UnreasonableRange,
    #[error("default outside choices")]
    DefaultOutsideChoices,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.as_ref().join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(entry.path(), to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

pub(crate) fn is_binary_buf(buf: &[u8]) -> bool {
    memchr(0u8, buf).is_some()
}

pub(crate) fn path_to_string<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .to_str()
        .expect("path encoding should be utf-8")
        .to_string()
}
