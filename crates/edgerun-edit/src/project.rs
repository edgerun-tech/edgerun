//! Project loading: discover all .rs source files from a Cargo.toml.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// All source files for a cargo project.
#[derive(Clone)]
pub struct Project {
    pub manifest: PathBuf,
    pub source_files: Vec<PathBuf>,
}

impl Project {
    /// Load a project from a Cargo.toml path by walking src/ directories.
    pub fn from_manifest(manifest: &Path) -> Result<Self, String> {
        let manifest = manifest
            .canonicalize()
            .map_err(|e| format!("Cargo.toml not found: {e}"))?;

        let mut files: HashSet<PathBuf> = HashSet::new();

        // Only walk the manifest directory's direct children that are likely workspace members
        if let Some(root) = manifest.parent() {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    // Skip build artifacts, VCS dirs, and dependency trees
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if matches!(name, "target" | ".git" | "node_modules" | ".cargo") {
                        continue;
                    }
                    if path.is_dir() {
                        let src = path.join("src");
                        if src.is_dir() {
                            walk_rs(&src, &mut files);
                        }
                    }
                }
            }
            // Also walk the root src/ directly
            let root_src = root.join("src");
            if root_src.is_dir() {
                walk_rs(&root_src, &mut files);
            }
        }

        // If no files found, just use manifest dir src/
        if files.is_empty() {
            if let Some(root) = manifest.parent() {
                let src = root.join("src");
                if src.is_dir() {
                    walk_rs(&src, &mut files);
                }
            }
        }

        Ok(Project {
            manifest,
            source_files: files.into_iter().collect(),
        })
    }

    /// Single file "project".
    pub fn single_file(path: &Path) -> Self {
        Project {
            manifest: path.parent().map(PathBuf::from).unwrap_or_default(),
            source_files: vec![path.to_path_buf()],
        }
    }

    pub fn root_dir(&self) -> &Path {
        self.manifest.parent().unwrap_or_else(|| Path::new("."))
    }
}

/// Walk a directory for all .rs files.
pub fn walk_rs(dir: &Path, out: &mut HashSet<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_rs(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.insert(path);
            }
        }
    }
}
