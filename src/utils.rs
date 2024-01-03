use std::path::Path;

use memchr::memchr;

// TODO: include path in std::io::Error
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
    #[error("cannot canonicalize base path")]
    CanonicalizeBasePath(#[source] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum InvalidVariableError {
    #[error("pattern with choices")]
    PatternWithChoices,
    #[error("unreasonable range")]
    UnreasonableRange,
    #[error("default mismatch pattern")]
    DefaultMismatchPattern,
    #[error("default outside choices")]
    DefaultOutsideChoices,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) fn is_binary_buf(buf: &[u8]) -> bool {
    memchr(0u8, buf).is_some()
}

pub(crate) fn path_to_string<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .to_str()
        .expect("path encoding should be utf-8")
        .to_string()
}
