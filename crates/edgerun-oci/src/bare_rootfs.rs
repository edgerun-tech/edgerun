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

#[cfg(all(test, not(target_os = "none")))]
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
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };

        rootfs.apply_entry(&entry, b"127.0.0.1").unwrap();

        assert!(rootfs.contains("etc"));
        assert_eq!(rootfs.read_file("etc/hosts"), Some(b"127.0.0.1".as_slice()));
    }

    #[test]
    fn rootfs_access_reads_ranges_and_hardlinks() {
        let mut rootfs = BareRootfs::new();
        let file = TarEntry {
            path: "bin/app".into(),
            kind: TarEntryKind::Regular,
            size: 10,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 7,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&file, b"0123456789").unwrap();
        let mut link = file.clone();
        link.path = "bin/run".into();
        link.kind = TarEntryKind::Hardlink;
        link.size = 0;
        link.link_name = Some("bin/app".into());
        rootfs.apply_entry(&link, &[]).unwrap();

        assert_eq!(OciRootfs::file_len(&rootfs, "/bin/run").unwrap(), Some(10));
        let mut buf = [0u8; 4];
        assert_eq!(
            OciRootfs::read_file_range(&rootfs, "/bin/run", 3, &mut buf).unwrap(),
            Some(4)
        );
        assert_eq!(&buf, b"3456");

        let entry = OciRootfs::entry(&rootfs, "bin/run").unwrap().unwrap();
        assert_eq!(entry.kind, OciRootfsEntryKind::Hardlink);
        assert_eq!(entry.link_name, Some("bin/app".into()));

        let bin_children = OciRootfs::children(&rootfs, "bin").unwrap();
        assert_eq!(
            bin_children
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["bin/app", "bin/run"]
        );

        let under_bin = OciRootfs::entries_under(&rootfs, "bin").unwrap();
        assert_eq!(under_bin.len(), 2);
    }

    #[test]
    fn rootfs_access_resolves_executables() {
        let mut rootfs = BareRootfs::new();
        let file = TarEntry {
            path: "usr/bin/app".into(),
            kind: TarEntryKind::Regular,
            size: 4,
            mode: 0o755,
            uid: 0,
            gid: 0,
            mtime: 0,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        };
        rootfs.apply_entry(&file, b"app!").unwrap();

        let mut symlink = file.clone();
        symlink.path = "bin/sh".into();
        symlink.kind = TarEntryKind::Symlink;
        symlink.size = 0;
        symlink.link_name = Some("../usr/bin/app".into());
        rootfs.apply_entry(&symlink, &[]).unwrap();

        let executable = crate::rootfs_access::resolve_executable_path(
            &rootfs,
            "app",
            &["PATH=/usr/bin:/bin".into()],
            "/",
        )
        .unwrap()
        .unwrap();
        assert_eq!(executable.path, "usr/bin/app");

        let executable =
            crate::rootfs_access::resolve_executable_path(&rootfs, "/bin/sh", &[], "/")
                .unwrap()
                .unwrap();
        assert_eq!(executable.path, "usr/bin/app");

        let executable =
            crate::rootfs_access::resolve_executable_path(&rootfs, "./app", &[], "/usr/bin")
                .unwrap()
                .unwrap();
        assert_eq!(executable.path, "usr/bin/app");

        let plan = crate::rootfs_access::build_launch_plan(
            &rootfs,
            &["app".into(), "--version".into()],
            &["PATH=/usr/bin".into(), "TERM=xterm".into()],
            "/",
        )
        .unwrap()
        .unwrap();
        assert_eq!(plan.executable.path, "usr/bin/app");
        assert_eq!(plan.argv, vec!["app", "--version"]);
        assert_eq!(plan.env, vec!["PATH=/usr/bin", "TERM=xterm"]);
        assert_eq!(plan.cwd, "/");
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
