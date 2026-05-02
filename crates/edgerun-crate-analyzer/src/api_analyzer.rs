use std::path::PathBuf;
use crate::crate_model::*;
use crate::rust_parser;

pub fn analyze_public_api(visible_files: &[PathBuf]) -> (Vec<ApiItem>, usize) {
    let (items, _edges) = rust_parser::analyze_source_files(visible_files);
    let public_items: Vec<ApiItem> = items.into_iter()
        .filter(|item| matches!(item.visibility, Visibility::Public))
        .collect();
    let count = public_items.len();
    (public_items, count)
}
