//! no_std in-memory OCI rootfs merge target.

use crate::oci_path::normalize_layer_path;
use crate::prelude::*;
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
                        data: Vec::new(),
                        link_name: None,
                    });
            }
            offset = end + 1;
        }
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
                data: entry_data,
                link_name: entry.link_name.clone(),
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_apply::apply_bare_image_layer_blobs;
    use crate::test_support::{
        bare_image_plan_for_layers as plan_for_layers, layer_descriptor as descriptor, tar,
        tar_entry, TestDigest,
    };

    #[test]
    fn stores_regular_files_and_parent_dirs() {
        let mut rootfs = BareRootfs::new();
        let entry = TarEntry {
            path: "etc/hosts".into(),
            kind: TarEntryKind::Regular,
            size: 9,
            mode: 0o644,
            uid: 0,
            gid: 0,
            link_name: None,
            whiteout: None,
        };

        rootfs.apply_entry(&entry, b"127.0.0.1").unwrap();

        assert!(rootfs.contains("etc"));
        assert_eq!(rootfs.read_file("etc/hosts"), Some(b"127.0.0.1".as_slice()));
    }

    #[test]
    fn applies_remove_whiteout() {
        let lower = tar(vec![tar_entry("etc/shadow", b'0', b"old")]);
        let upper = tar(vec![tar_entry("etc/.wh.shadow", b'0', b"")]);
        let plan = plan_for_layers(vec![descriptor(&lower), descriptor(&upper)]);
        let mut rootfs = BareRootfs::new();

        apply_bare_image_layer_blobs(
            &plan,
            [lower.as_slice(), upper.as_slice()],
            |_| TestDigest::default(),
            &mut rootfs,
        )
        .unwrap();

        assert!(!rootfs.contains("etc/shadow"));
    }

    #[test]
    fn applies_opaque_directory_whiteout() {
        let lower = tar(vec![
            tar_entry("var/cache/old", b'0', b"old"),
            tar_entry("var/lib/keep", b'0', b"keep"),
        ]);
        let upper = tar(vec![
            tar_entry("var/cache/.wh..wh..opq", b'0', b""),
            tar_entry("var/cache/new", b'0', b"new"),
        ]);
        let plan = plan_for_layers(vec![descriptor(&lower), descriptor(&upper)]);
        let mut rootfs = BareRootfs::new();

        apply_bare_image_layer_blobs(
            &plan,
            [lower.as_slice(), upper.as_slice()],
            |_| TestDigest::default(),
            &mut rootfs,
        )
        .unwrap();

        assert!(!rootfs.contains("var/cache/old"));
        assert_eq!(rootfs.read_file("var/cache/new"), Some(b"new".as_slice()));
        assert_eq!(rootfs.read_file("var/lib/keep"), Some(b"keep".as_slice()));
    }
}
