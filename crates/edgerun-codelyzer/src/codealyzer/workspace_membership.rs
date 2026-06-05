use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Default)]
pub struct WorkspaceMembershipReport {
    pub workspace_root: PathBuf,
    pub discovered_package_manifests: Vec<PathBuf>,
    pub member_package_manifests: Vec<PathBuf>,
    pub missing_package_manifests: Vec<PathBuf>,
}

impl WorkspaceMembershipReport {
    pub fn is_complete(&self) -> bool {
        self.missing_package_manifests.is_empty()
    }
}

pub fn check_workspace_membership(
    workspace_root: &Path,
) -> Result<WorkspaceMembershipReport, String> {
    let workspace_root = workspace_root
        .canonicalize()
        .map_err(|err| format!("invalid workspace root: {err}"))?;
    let discovered = discover_package_manifests(&workspace_root.join("crates"))?;
    let members = workspace_member_manifests(&workspace_root)?;
    let member_set = members.iter().cloned().collect::<BTreeSet<_>>();

    let missing = discovered
        .iter()
        .filter(|manifest| !member_set.contains(*manifest))
        .cloned()
        .collect::<Vec<_>>();

    Ok(WorkspaceMembershipReport {
        workspace_root,
        discovered_package_manifests: discovered,
        member_package_manifests: members,
        missing_package_manifests: missing,
    })
}

fn discover_package_manifests(crates_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut manifests = Vec::new();
    if !crates_root.exists() {
        return Ok(manifests);
    }
    let mut stack = vec![crates_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|err| format!("read {} failed: {err}", dir.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if should_skip_dir(&path) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml")
                && manifest_has_package(&path)?
            {
                manifests.push(canonical_path(&path)?);
            }
        }
    }
    manifests.sort();
    manifests.dedup();
    Ok(manifests)
}

fn workspace_member_manifests(workspace_root: &Path) -> Result<Vec<PathBuf>, String> {
    let manifest_path = workspace_root.join("Cargo.toml");
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            manifest_path
                .to_str()
                .ok_or_else(|| "workspace manifest path is not utf-8".to_string())?,
        ])
        .output()
        .map_err(|err| format!("cargo metadata failed to start: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let metadata = std::str::from_utf8(&output.stdout)
        .map_err(|err| format!("cargo metadata output was not utf-8: {err}"))?;
    let workspace_members = fixed_json_string_array_field(metadata, "workspace_members")
        .ok_or_else(|| "cargo metadata missing workspace_members".to_string())?
        .into_iter()
        .collect::<BTreeSet<_>>();

    let packages = fixed_json_object_array_field(metadata, "packages")
        .ok_or_else(|| "cargo metadata missing packages".to_string())?;
    let mut manifests = Vec::new();
    for package in packages {
        let Some(id) = fixed_json_string_field(package, "id") else {
            continue;
        };
        if !workspace_members.contains(&id) {
            continue;
        }
        let Some(manifest) = fixed_json_string_field(package, "manifest_path") else {
            continue;
        };
        manifests.push(canonical_path(Path::new(manifest))?);
    }
    manifests.sort();
    manifests.dedup();
    Ok(manifests)
}

fn should_skip_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | "target" | "node_modules")
    )
}

fn manifest_has_package(path: &Path) -> Result<bool, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|err| format!("read {} failed: {err}", path.display()))?;
    Ok(content
        .lines()
        .map(str::trim)
        .any(|line| line == "[package]"))
}

fn canonical_path(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|err| format!("canonicalize {} failed: {err}", path.display()))
}

fn fixed_json_string_field(input: &str, key: &str) -> Option<String> {
    let value = fixed_json_field_value(input, key)?;
    parse_json_string(value.trim_start()).map(|(value, _)| value)
}

fn fixed_json_string_array_field(input: &str, key: &str) -> Option<Vec<String>> {
    let value = fixed_json_field_value(input, key)?.trim_start();
    if !value.starts_with('[') {
        return None;
    }
    let end = matching_json_end(value, '[', ']')?;
    let array = &value[1..end - 1];
    let mut items = Vec::new();
    let mut rest = array.trim_start();
    while !rest.is_empty() {
        let (item, used) = parse_json_string(rest)?;
        items.push(item);
        rest = rest[used..].trim_start();
        if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        } else if !rest.is_empty() {
            return None;
        }
    }
    Some(items)
}

fn fixed_json_object_array_field<'a>(input: &'a str, key: &str) -> Option<Vec<&'a str>> {
    let value = fixed_json_field_value(input, key)?.trim_start();
    if !value.starts_with('[') {
        return None;
    }
    let end = matching_json_end(value, '[', ']')?;
    let mut rest = value[1..end - 1].trim_start();
    let mut objects = Vec::new();
    while !rest.is_empty() {
        if !rest.starts_with('{') {
            return None;
        }
        let object_end = matching_json_end(rest, '{', '}')?;
        objects.push(&rest[..object_end]);
        rest = rest[object_end..].trim_start();
        if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        } else if !rest.is_empty() {
            return None;
        }
    }
    Some(objects)
}

fn fixed_json_field_value<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let mut rest = input;
    while let Some(pos) = rest.find('"') {
        rest = &rest[pos..];
        let (found, used) = parse_json_string(rest)?;
        rest = &rest[used..];
        let after_key = rest.trim_start();
        if !after_key.starts_with(':') {
            continue;
        }
        let value = after_key[1..].trim_start();
        if found == key {
            return Some(value);
        }
        rest = value;
    }
    None
}

fn parse_json_string(input: &str) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    if bytes.first().copied() != Some(b'"') {
        return None;
    }
    let mut out = String::new();
    let mut i = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Some((out, i + 1)),
            b'\\' => {
                i += 1;
                let escaped = *bytes.get(i)?;
                match escaped {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = input.get(i + 1..i + 5)?;
                        let scalar = u16::from_str_radix(hex, 16).ok()?;
                        out.push(char::from_u32(scalar as u32)?);
                        i += 4;
                    }
                    _ => return None,
                }
            }
            byte => out.push(byte as char),
        }
        i += 1;
    }
    None
}

fn matching_json_end(input: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(idx + ch.len_utf8());
            }
        }
    }
    None
}
