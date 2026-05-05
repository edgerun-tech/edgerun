//! no_std in-memory OCI rootfs merge target.

use crate::oci_path::normalize_layer_path;
use crate::prelude::*;
use crate::rootfs_access::{
    normalize_rootfs_path, OciDeviceId, OciRootfs, OciRootfsEntry, OciRootfsEntryKind,
    OciRootfsError,
};
use crate::tar_layer::{OciWhiteout, TarEntry, TarEntryKind, TarLayerSink};
use alloc::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BareRootfs {
    entries: BTreeMap<String, BareRootfsEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BareRootfsEntry {
    pub kind: BareRootfsEntryKind,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: u64,
    pub dev_major: Option<u32>,
    pub dev_minor: Option<u32>,
    pub data: Vec<u8>,
    pub link_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BareRootfsEntryKind {
    Regular,
    Directory,
    Symlink,
    Hardlink,
    Character,
    Block,
    Fifo,
}

impl Default for BareRootfs {
    fn default() -> Self {
        Self::new()
    }
}

impl BareRootfs {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, path: &str) -> bool {
        self.entries
            .contains_key(normalize_layer_path(path).as_str())
    }

    pub fn get(&self, path: &str) -> Option<&BareRootfsEntry> {
        self.entries.get(normalize_layer_path(path).as_str())
    }

    pub fn read_file(&self, path: &str) -> Option<&[u8]> {
        let entry = self.get(path)?;
        (entry.kind == BareRootfsEntryKind::Regular).then_some(entry.data.as_slice())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &BareRootfsEntry)> {
        self.entries
            .iter()
            .map(|(path, entry)| (path.as_str(), entry))
    }

    fn resolve_file_entry(&self, path: &str) -> Result<Option<&BareRootfsEntry>, OciRootfsError> {
        let original = normalize_rootfs_path(path, true)?;
        let mut current = original.clone();

        for _ in 0..=self.entries.len() {
            let Some(entry) = self.entries.get(current.as_str()) else {
                return Ok(None);
            };

            match entry.kind {
                BareRootfsEntryKind::Regular => return Ok(Some(entry)),
                BareRootfsEntryKind::Hardlink => {
                    let Some(target) = entry.link_name.as_deref() else {
                        return Ok(None);
                    };
                    current = normalize_rootfs_path(target, false)?;
                }
                _ => return Ok(None),
            }
        }

        Err(OciRootfsError::LinkLoop(original))
    }

    pub fn remove_path(&mut self, path: &str) {
        let path = normalize_layer_path(path);
        self.entries.remove(path.as_str());
        self.remove_children(path.as_str());
    }

    pub fn remove_children(&mut self, path: &str) {
        let path = normalize_layer_path(path);
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}/")
        };
        self.entries
            .retain(|entry_path, _| !entry_path.starts_with(prefix.as_str()));
    }

    pub fn apply_whiteout(&mut self, whiteout: &OciWhiteout) {
        match whiteout {
            OciWhiteout::RemovePath(path) => self.remove_path(path),
            OciWhiteout::OpaqueDirectory(path) => self.remove_children(path),
        }
    }

    fn ensure_parent_dirs(&mut self, path: &str, uid: u32, gid: u32) {
        let mut offset = 0usize;
        while let Some(relative) = path[offset..].find('/') {
            let end = offset + relative;
            let parent = &path[..end];
            if !parent.is_empty() {
                self.entries
                    .entry(parent.into())
                    .or_insert_with(|| BareRootfsEntry {
                        kind: BareRootfsEntryKind::Directory,
                        mode: 0o755,
                        uid,
                        gid,
                        mtime: 0,
                        dev_major: None,
                        dev_minor: None,
                        data: Vec::new(),
                        link_name: None,
                    });
            }
            offset = end + 1;
        }
    }
}

impl OciRootfs for BareRootfs {
    fn entry(&self, path: &str) -> Result<Option<OciRootfsEntry>, OciRootfsError> {
        let path = normalize_rootfs_path(path, true)?;
        Ok(self
            .entries
            .get_key_value(path.as_str())
            .map(|(path, entry)| rootfs_entry(path, entry)))
    }

    fn file_len(&self, path: &str) -> Result<Option<usize>, OciRootfsError> {
        Ok(self.resolve_file_entry(path)?.map(|entry| entry.data.len()))
    }

    fn read_file_range(
        &self,
        path: &str,
        offset: u64,
        out: &mut [u8],
    ) -> Result<Option<usize>, OciRootfsError> {
        let Some(entry) = self.resolve_file_entry(path)? else {
            return Ok(None);
        };
        let offset = usize::try_from(offset)
            .map_err(|_| OciRootfsError::Backend("file offset exceeds usize".into()))?;
        if offset >= entry.data.len() || out.is_empty() {
            return Ok(Some(0));
        }

        let count = core::cmp::min(out.len(), entry.data.len() - offset);
        out[..count].copy_from_slice(&entry.data[offset..offset + count]);
        Ok(Some(count))
    }

    fn read_link(&self, path: &str) -> Result<Option<String>, OciRootfsError> {
        let path = normalize_rootfs_path(path, true)?;
        Ok(self.entries.get(path.as_str()).and_then(|entry| {
            matches!(
                entry.kind,
                BareRootfsEntryKind::Symlink | BareRootfsEntryKind::Hardlink
            )
            .then(|| entry.link_name.clone())
            .flatten()
        }))
    }

    fn read_device(&self, path: &str) -> Result<Option<OciDeviceId>, OciRootfsError> {
        let path = normalize_rootfs_path(path, true)?;
        Ok(self.entries.get(path.as_str()).and_then(|entry| {
            match (entry.dev_major, entry.dev_minor) {
                (Some(major), Some(minor))
                    if matches!(
                        entry.kind,
                        BareRootfsEntryKind::Character | BareRootfsEntryKind::Block
                    ) =>
                {
                    Some(OciDeviceId { major, minor })
                }
                _ => None,
            }
        }))
    }

    fn entries_under(&self, path: &str) -> Result<Vec<OciRootfsEntry>, OciRootfsError> {
        let path = normalize_rootfs_path(path, true)?;
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}/")
        };

        Ok(self
            .entries
            .iter()
            .filter(|(entry_path, _)| prefix.is_empty() || entry_path.starts_with(prefix.as_str()))
            .map(|(path, entry)| rootfs_entry(path, entry))
            .collect())
    }

    fn children(&self, path: &str) -> Result<Vec<OciRootfsEntry>, OciRootfsError> {
        let path = normalize_rootfs_path(path, true)?;
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}/")
        };

        Ok(self
            .entries
            .iter()
            .filter(|(entry_path, _)| {
                let relative = if prefix.is_empty() {
                    entry_path.as_str()
                } else if let Some(relative) = entry_path.strip_prefix(prefix.as_str()) {
                    relative
                } else {
                    return false;
                };
                !relative.is_empty() && !relative.contains('/')
            })
            .map(|(path, entry)| rootfs_entry(path, entry))
            .collect())
    }
}

fn rootfs_entry(path: &str, entry: &BareRootfsEntry) -> OciRootfsEntry {
    OciRootfsEntry {
        path: path.into(),
        kind: match entry.kind {
            BareRootfsEntryKind::Regular => OciRootfsEntryKind::Regular,
            BareRootfsEntryKind::Directory => OciRootfsEntryKind::Directory,
            BareRootfsEntryKind::Symlink => OciRootfsEntryKind::Symlink,
            BareRootfsEntryKind::Hardlink => OciRootfsEntryKind::Hardlink,
            BareRootfsEntryKind::Character => OciRootfsEntryKind::Character,
            BareRootfsEntryKind::Block => OciRootfsEntryKind::Block,
            BareRootfsEntryKind::Fifo => OciRootfsEntryKind::Fifo,
        },
        mode: entry.mode,
        uid: entry.uid,
        gid: entry.gid,
        mtime: entry.mtime,
        len: entry.data.len(),
        link_name: entry.link_name.clone(),
        device: match (entry.dev_major, entry.dev_minor) {
            (Some(major), Some(minor)) => Some(OciDeviceId { major, minor }),
            _ => None,
        },
    }
}

impl TarLayerSink for BareRootfs {
    fn apply_entry(&mut self, entry: &TarEntry, data: &[u8]) -> Result<(), String> {
        if let Some(whiteout) = entry.whiteout.as_ref() {
            self.apply_whiteout(whiteout);
            return Ok(());
        }

        let path = normalize_layer_path(&entry.path);
        if path.is_empty() {
            return Err("empty rootfs path".into());
        }

        self.ensure_parent_dirs(path.as_str(), entry.uid, entry.gid);
        let kind = match entry.kind {
            TarEntryKind::Regular => BareRootfsEntryKind::Regular,
            TarEntryKind::Directory => BareRootfsEntryKind::Directory,
            TarEntryKind::Symlink => BareRootfsEntryKind::Symlink,
            TarEntryKind::Hardlink => BareRootfsEntryKind::Hardlink,
            TarEntryKind::Character => BareRootfsEntryKind::Character,
            TarEntryKind::Block => BareRootfsEntryKind::Block,
            TarEntryKind::Fifo => BareRootfsEntryKind::Fifo,
            TarEntryKind::PaxExtended
            | TarEntryKind::PaxGlobal
            | TarEntryKind::GnuLongName
            | TarEntryKind::GnuLongLink => return Ok(()),
        };

        let entry_data = if kind == BareRootfsEntryKind::Regular {
            data.to_vec()
        } else {
            Vec::new()
        };

        self.entries.insert(
            path,
            BareRootfsEntry {
                kind,
                mode: entry.mode,
                uid: entry.uid,
                gid: entry.gid,
                mtime: entry.mtime,
                dev_major: entry.dev_major,
                dev_minor: entry.dev_minor,
                data: entry_data,
                link_name: entry.link_name.clone(),
            },
        );

        Ok(())
    }
}
