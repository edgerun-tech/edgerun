//! OCI layer path normalization and safety checks.

use crate::prelude::*;

pub fn normalize_layer_path(path: &str) -> String {
    let mut out = String::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            _ => {
                if !out.is_empty() {
                    out.push('/');
                }
                out.push_str(component);
            }
        }
    }
    out
}

pub fn layer_path_safe(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.contains('\0') {
        return false;
    }
    let mut depth = 0usize;
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            _ => depth += 1,
        }
    }
    true
}
