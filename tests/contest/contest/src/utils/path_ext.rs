//! Path extension utilities — replaces `PathBufExt` from youki's `libcontainer`.

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PathBufExtError {
    #[error("relative path cannot be converted to the path in the container")]
    RelativePath,
    #[error("failed to strip prefix from {path:?}")]
    StripPrefix {
        path: PathBuf,
        source: std::path::StripPrefixError,
    },
    #[error("failed to canonicalize path {path:?}")]
    Canonicalize { path: PathBuf, source: std::io::Error },
    #[error("failed to get current directory")]
    CurrentDir { source: std::io::Error },
}

/// Extension trait for safe path joining.
pub trait PathBufExt {
    /// Join a path while preventing path traversal.
    ///
    /// If `path` is absolute, strips the leading `/` before joining.
    /// If `path` is relative, does a normal join.
    fn join_safely<P: AsRef<Path>>(&self, p: P) -> Result<PathBuf, PathBufExtError>;
}

impl PathBufExt for PathBuf {
    fn join_safely<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, PathBufExtError> {
        let path = path.as_ref();
        if path.is_relative() {
            return Ok(self.join(path));
        }

        let stripped = path
            .strip_prefix("/")
            .map_err(|e| PathBufExtError::StripPrefix {
                path: self.to_path_buf(),
                source: e,
            })?;
        Ok(self.join(stripped))
    }
}
