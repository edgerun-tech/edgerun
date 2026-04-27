//! no_std rootfs access abstraction for bare runtimes.

use crate::prelude::*;
use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OciRootfsError {
    InvalidPath(String),
    Backend(String),
    LinkLoop(String),
    EmptyCommand,
}

impl fmt::Display for OciRootfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath(path) => write!(f, "invalid rootfs path: {path}"),
            Self::Backend(error) => write!(f, "rootfs backend error: {error}"),
            Self::LinkLoop(path) => write!(f, "rootfs hardlink loop at path: {path}"),
            Self::EmptyCommand => f.write_str("empty executable command"),
        }
    }
}

impl core::error::Error for OciRootfsError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciRootfsEntryKind {
    Regular,
    Directory,
    Symlink,
    Hardlink,
    Character,
    Block,
    Fifo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OciDeviceId {
    pub major: u32,
    pub minor: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciRootfsEntry {
    pub path: String,
    pub kind: OciRootfsEntryKind,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: u64,
    pub len: usize,
    pub link_name: Option<String>,
    pub device: Option<OciDeviceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciExecutable {
    pub path: String,
    pub entry: OciRootfsEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciLaunchPlan {
    pub executable: OciExecutable,
    pub argv: Vec<String>,
    pub env: Vec<String>,
    pub cwd: String,
}

pub trait OciRootfs {
    fn entry(&self, path: &str) -> Result<Option<OciRootfsEntry>, OciRootfsError>;

    fn file_len(&self, path: &str) -> Result<Option<usize>, OciRootfsError>;

    fn read_file_range(
        &self,
        path: &str,
        offset: u64,
        out: &mut [u8],
    ) -> Result<Option<usize>, OciRootfsError>;

    fn read_link(&self, path: &str) -> Result<Option<String>, OciRootfsError>;

    fn read_device(&self, path: &str) -> Result<Option<OciDeviceId>, OciRootfsError>;

    fn entries_under(&self, path: &str) -> Result<Vec<OciRootfsEntry>, OciRootfsError>;

    fn children(&self, path: &str) -> Result<Vec<OciRootfsEntry>, OciRootfsError>;

    fn contains(&self, path: &str) -> Result<bool, OciRootfsError> {
        Ok(self.entry(path)?.is_some())
    }
}

pub fn build_launch_plan<R: OciRootfs>(
    rootfs: &R,
    args: &[String],
    env: &[String],
    cwd: &str,
) -> Result<Option<OciLaunchPlan>, OciRootfsError> {
    let executable = match resolve_executable(rootfs, args, env, cwd)? {
        Some(executable) => executable,
        None => return Ok(None),
    };
    let cwd = normalize_rootfs_path(cwd, true)?;

    Ok(Some(OciLaunchPlan {
        executable,
        argv: args.to_vec(),
        env: env.to_vec(),
        cwd: if cwd.is_empty() { "/".into() } else { cwd },
    }))
}

pub fn resolve_executable<R: OciRootfs>(
    rootfs: &R,
    args: &[String],
    env: &[String],
    cwd: &str,
) -> Result<Option<OciExecutable>, OciRootfsError> {
    let Some(command) = args.first() else {
        return Err(OciRootfsError::EmptyCommand);
    };
    resolve_executable_path(rootfs, command, env, cwd)
}

pub fn resolve_executable_path<R: OciRootfs>(
    rootfs: &R,
    command: &str,
    env: &[String],
    cwd: &str,
) -> Result<Option<OciExecutable>, OciRootfsError> {
    if command.is_empty() {
        return Err(OciRootfsError::EmptyCommand);
    }

    if command.contains('/') {
        let path = if command.starts_with('/') {
            normalize_rootfs_path(command, false)?
        } else {
            join_rootfs_path(cwd, command)?
        };
        return executable_at(rootfs, path.as_str());
    }

    let path = path_env(env);
    for dir in path.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = join_rootfs_path(dir, command)?;
        if let Some(executable) = executable_at(rootfs, candidate.as_str())? {
            return Ok(Some(executable));
        }
    }

    Ok(None)
}

fn executable_at<R: OciRootfs>(
    rootfs: &R,
    path: &str,
) -> Result<Option<OciExecutable>, OciRootfsError> {
    let Some((path, entry)) = resolve_entry_following_symlinks(rootfs, path)? else {
        return Ok(None);
    };
    if !matches!(
        entry.kind,
        OciRootfsEntryKind::Regular | OciRootfsEntryKind::Hardlink
    ) {
        return Ok(None);
    }
    if entry.mode & 0o111 == 0 || rootfs.file_len(path.as_str())?.is_none() {
        return Ok(None);
    }
    Ok(Some(OciExecutable { path, entry }))
}

fn resolve_entry_following_symlinks<R: OciRootfs>(
    rootfs: &R,
    path: &str,
) -> Result<Option<(String, OciRootfsEntry)>, OciRootfsError> {
    let original = normalize_rootfs_path(path, false)?;
    let mut current = original.clone();

    for _ in 0..=32 {
        let Some(entry) = rootfs.entry(current.as_str())? else {
            return Ok(None);
        };
        if entry.kind != OciRootfsEntryKind::Symlink {
            return Ok(Some((current, entry)));
        }

        let Some(target) = entry.link_name.as_deref() else {
            return Ok(None);
        };
        current = if target.starts_with('/') {
            normalize_rootfs_path(target, false)?
        } else {
            let parent = current
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .unwrap_or("");
            join_rootfs_path(parent, target)?
        };
    }

    Err(OciRootfsError::LinkLoop(original))
}

fn path_env(env: &[String]) -> &str {
    env.iter()
        .find_map(|value| value.strip_prefix("PATH="))
        .unwrap_or("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
}

fn join_rootfs_path(base: &str, child: &str) -> Result<String, OciRootfsError> {
    if child.starts_with('/') {
        return normalize_rootfs_path(child, false);
    }

    let base = normalize_rootfs_path(base, true)?;
    let joined = if base.is_empty() {
        child.into()
    } else {
        format!("{base}/{child}")
    };
    normalize_rootfs_path(joined.as_str(), false)
}

pub(crate) fn normalize_rootfs_path(
    path: &str,
    allow_empty: bool,
) -> Result<String, OciRootfsError> {
    if path.contains('\0') {
        return Err(OciRootfsError::InvalidPath(path.into()));
    }

    let mut out = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if out.pop().is_none() {
                    return Err(OciRootfsError::InvalidPath(path.into()));
                }
            }
            other => out.push(other),
        }
    }

    let normalized = out.join("/");
    if !allow_empty && normalized.is_empty() {
        return Err(OciRootfsError::InvalidPath(path.into()));
    }
    Ok(normalized)
}
