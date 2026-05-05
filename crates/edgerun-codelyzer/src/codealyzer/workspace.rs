use std::path::{Path, PathBuf};
use crate::codealyzer::errors::Result;

pub fn find_crate_in_workspace(name: &str, workspace_root: &Path) -> Result<PathBuf> {
    let crates_dir = workspace_root.join("crates");
    if !crates_dir.exists() {
        return Err(crate::codealyzer::errors::AnalyzerError::CrateNotFound(name.into()));
    }

    let candidate = crates_dir.join(name);
    if candidate.exists() && candidate.is_dir() {
        return Ok(candidate);
    }

    let entries = std::fs::read_dir(&crates_dir)
        .map_err(|e| crate::codealyzer::errors::AnalyzerError::IoError(e.to_string()))?;

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.file_name().map(|n| n.to_string_lossy()) == Some(std::borrow::Cow::Borrowed(name)) {
            return Ok(path);
        }
    }

    Err(crate::codealyzer::errors::AnalyzerError::CrateNotFound(name.into()))
}

pub fn get_workspace_root() -> PathBuf {
    let mut current = std::env::current_dir().unwrap_or_default();
    while current.parent().is_some() {
        if current.join("Cargo.toml").exists() {
            if let Ok(content) = std::fs::read_to_string(current.join("Cargo.toml")) {
                if content.contains("[workspace]") {
                    return current;
                }
            }
        }
        current = current.parent().unwrap().to_path_buf();
    }
    std::env::current_dir().unwrap_or_default()
}
