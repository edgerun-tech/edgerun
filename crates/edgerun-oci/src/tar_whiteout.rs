use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OciWhiteout {
    RemovePath(String),
    OpaqueDirectory(String),
}

pub fn parse_oci_whiteout(path: &str) -> Option<OciWhiteout> {
    let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
    if name == ".wh..wh..opq" {
        return Some(OciWhiteout::OpaqueDirectory(parent.into()));
    }
    name.strip_prefix(".wh.").map(|target| {
        let target_path = if parent.is_empty() {
            target.into()
        } else {
            format!("{parent}/{target}")
        };
        OciWhiteout::RemovePath(target_path)
    })
}
