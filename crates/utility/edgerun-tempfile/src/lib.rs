use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

#[derive(Debug)]
pub struct NamedTempFile {
    path: PathBuf,
}

impl TempDir {
    pub fn new() -> std::io::Result<Self> {
        let base = std::env::temp_dir();
        for _ in 0..128 {
            let path = base.join(format!(
                "edgerun-temp-{}-{}",
                std::process::id(),
                edgerun_random::u64()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "failed to create unique temp directory",
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub fn tempdir() -> std::io::Result<TempDir> {
    TempDir::new()
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

impl NamedTempFile {
    pub fn new() -> std::io::Result<Self> {
        let base = std::env::temp_dir();
        for _ in 0..128 {
            let path = base.join(format!(
                "edgerun-temp-file-{}-{}",
                std::process::id(),
                edgerun_random::u64()
            ));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "failed to create unique temp file",
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for NamedTempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
