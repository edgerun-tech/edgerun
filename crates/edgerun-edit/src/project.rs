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
    /// Load a project from a Cargo.toml path using cargo metadata + filesystem walk.
    pub fn from_manifest(manifest: &Path) -> Self {
        let manifest = manifest
            .canonicalize()
            .unwrap_or_else(|e| panic!("Cargo.toml not found: {e}"));

        let mut files: HashSet<PathBuf> = HashSet::new();

        // Walk src/ dirs from workspace root for completeness
        if let Some(root) = manifest.parent() {
            for entry in std::fs::read_dir(root).ok().into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let src = path.join("src");
                    if src.is_dir() {
                        walk_rs(&src, &mut files);
                    }
                }
            }
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

        Project {
            manifest,
            source_files: files.into_iter().collect(),
        }
    }

    /// Single file "project".
    pub fn single_file(path: &Path) -> Self {
        Project {
            manifest: path.parent().map(PathBuf::from).unwrap_or_default(),
            source_files: vec![path.to_path_buf()],
        }
    }

    pub fn root_dir(&self) -> &Path {
        self.manifest.parent().unwrap()
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
