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

    let metadata = edgerun_json::from_slice(&output.stdout)
        .map_err(|err| format!("parse cargo metadata failed: {err}"))?;
    let workspace_members = metadata
        .get("workspace_members")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "cargo metadata missing workspace_members".to_string())?
        .iter()
        .filter_map(|member| member.as_str())
        .collect::<BTreeSet<_>>();

    let packages = metadata
        .get("packages")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "cargo metadata missing packages".to_string())?;
    let mut manifests = Vec::new();
    for package in packages {
        let Some(id) = package.get("id").and_then(|value| value.as_str()) else {
            continue;
        };
        if !workspace_members.contains(id) {
            continue;
        }
        let Some(manifest) = package
            .get("manifest_path")
            .and_then(|value| value.as_str())
        else {
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
