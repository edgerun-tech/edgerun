use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;

pub struct ArtifactWriter {
    dir: PathBuf,
}

impl ArtifactWriter {
    pub fn new(dir: &str) -> Result<Self> {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn write(&mut self, name: &str, content: &str) -> Result<()> {
        let path = self.dir.join(name);
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn read(&self, name: &str) -> Result<String> {
        let path = self.dir.join(name);
        Ok(fs::read_to_string(&path)?)
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}