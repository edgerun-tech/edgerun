use crate::codealyzer::gitvisible::*;
use std::path::{Path, PathBuf};

pub fn index_crate_sources(crate_dir: &Path) -> (Vec<PathBuf>, usize) {
    let policy = load_visibility_policy(crate_dir);
    collect_visible_files(crate_dir, &policy)
}
