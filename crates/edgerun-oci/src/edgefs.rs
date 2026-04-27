//! EdgeFS-backed no_std OCI layer sink.

use crate::oci_path::normalize_layer_path;
use crate::prelude::*;
use crate::rootfs_access::{
    OciDeviceId, OciRootfs, OciRootfsEntry, OciRootfsEntryKind, OciRootfsError,
};
use crate::tar_layer::{OciWhiteout, TarEntry, TarEntryKind, TarLayerSink};
use edgerun_edgefs::{DeviceId, EdgeFs, EntryKind, EntryRef, FileMeta};
use edgerun_storage::BlockStorage;

pub struct EdgeFsLayerSink<S: BlockStorage> {
    fs: EdgeFs<S>,
}

impl<S: BlockStorage> EdgeFsLayerSink<S> {
    pub fn new(fs: EdgeFs<S>) -> Self {
        Self { fs }
    }

    pub fn filesystem(&self) -> &EdgeFs<S> {
        &self.fs
    }

    pub fn filesystem_mut(&mut self) -> &mut EdgeFs<S> {
        &mut self.fs
    }

    pub fn into_filesystem(self) -> EdgeFs<S> {
        self.fs
    }
}

impl<S: BlockStorage> TarLayerSink for EdgeFsLayerSink<S> {
    fn apply_entry(&mut self, entry: &TarEntry, data: &[u8]) -> Result<(), String> {
        apply_entry_to_edgefs(&mut self.fs, entry, data)
    }
}

impl<S: BlockStorage> TarLayerSink for EdgeFs<S> {
    fn apply_entry(&mut self, entry: &TarEntry, data: &[u8]) -> Result<(), String> {
        apply_entry_to_edgefs(self, entry, data)
    }
}

impl<S: BlockStorage> OciRootfs for EdgeFs<S> {
    fn entry(&self, path: &str) -> Result<Option<OciRootfsEntry>, OciRootfsError> {
        self.entry(path)
            .map(|entry| entry.map(edgefs_entry))
            .map_err(edgefs_error)
    }

    fn file_len(&self, path: &str) -> Result<Option<usize>, OciRootfsError> {
        self.file_len(path).map_err(edgefs_error)
    }

    fn read_file_range(
        &self,
        path: &str,
        offset: u64,
        out: &mut [u8],
    ) -> Result<Option<usize>, OciRootfsError> {
        self.read_file_range(path, offset, out)
            .map_err(edgefs_error)
    }

    fn read_link(&self, path: &str) -> Result<Option<String>, OciRootfsError> {
        self.read_link(path).map_err(edgefs_error)
    }

    fn read_device(&self, path: &str) -> Result<Option<OciDeviceId>, OciRootfsError> {
        self.read_device(path)
            .map(|device| {
                device.map(|device| OciDeviceId {
                    major: device.major,
                    minor: device.minor,
                })
            })
            .map_err(edgefs_error)
    }

    fn entries_under(&self, path: &str) -> Result<Vec<OciRootfsEntry>, OciRootfsError> {
        self.entries_under(path)
            .map(|entries| entries.map(edgefs_entry).collect())
            .map_err(edgefs_error)
    }

    fn children(&self, path: &str) -> Result<Vec<OciRootfsEntry>, OciRootfsError> {
        self.children(path)
            .map(|entries| entries.map(edgefs_entry).collect())
            .map_err(edgefs_error)
    }
}

fn edgefs_entry(entry: EntryRef<'_>) -> OciRootfsEntry {
    OciRootfsEntry {
        path: entry.path.into(),
        kind: match entry.kind {
            EntryKind::File => OciRootfsEntryKind::Regular,
            EntryKind::Directory => OciRootfsEntryKind::Directory,
            EntryKind::Symlink => OciRootfsEntryKind::Symlink,
            EntryKind::Hardlink => OciRootfsEntryKind::Hardlink,
            EntryKind::Character => OciRootfsEntryKind::Character,
            EntryKind::Block => OciRootfsEntryKind::Block,
            EntryKind::Fifo => OciRootfsEntryKind::Fifo,
        },
        mode: entry.meta.mode,
        uid: entry.meta.uid,
        gid: entry.meta.gid,
        mtime: entry.meta.mtime,
        len: entry.len(),
        link_name: entry.link_target.map(String::from),
        device: entry.device.map(|device| OciDeviceId {
            major: device.major,
            minor: device.minor,
        }),
    }
}

fn edgefs_error(error: edgerun_edgefs::EdgeFsError) -> OciRootfsError {
    match error {
        edgerun_edgefs::EdgeFsError::InvalidPath(path) => OciRootfsError::InvalidPath(path),
        edgerun_edgefs::EdgeFsError::HardlinkLoop(path) => OciRootfsError::LinkLoop(path),
        other => OciRootfsError::Backend(other.to_string()),
    }
}

fn apply_entry_to_edgefs<S: BlockStorage>(
    fs: &mut EdgeFs<S>,
    entry: &TarEntry,
    data: &[u8],
) -> Result<(), String> {
    if let Some(whiteout) = entry.whiteout.as_ref() {
        return apply_whiteout_to_edgefs(fs, whiteout);
    }

    let path = normalize_layer_path(&entry.path);
    if path.is_empty() {
        return Err("empty rootfs path".into());
    }

    let meta = FileMeta::file(entry.mode, entry.uid, entry.gid).with_mtime(entry.mtime);
    match entry.kind {
        TarEntryKind::Regular => fs.write_file(path.as_str(), data, meta),
        TarEntryKind::Directory => fs.mkdir_all(path.as_str(), meta),
        TarEntryKind::Symlink => {
            let target = entry
                .link_name
                .as_deref()
                .ok_or_else(|| format!("missing symlink target for {}", entry.path))?;
            fs.symlink(path.as_str(), target, meta)
        }
        TarEntryKind::Hardlink => {
            let target = entry
                .link_name
                .as_deref()
                .ok_or_else(|| format!("missing hardlink target for {}", entry.path))?;
            fs.hardlink(path.as_str(), target, meta)
        }
        TarEntryKind::Character => {
            let device = device_id(entry)?;
            fs.create_device_node(path.as_str(), EntryKind::Character, meta, device)
        }
        TarEntryKind::Block => {
            let device = device_id(entry)?;
            fs.create_device_node(path.as_str(), EntryKind::Block, meta, device)
        }
        TarEntryKind::Fifo => fs.create_node(path.as_str(), EntryKind::Fifo, meta),
        TarEntryKind::PaxExtended
        | TarEntryKind::PaxGlobal
        | TarEntryKind::GnuLongName
        | TarEntryKind::GnuLongLink => Ok(()),
    }
    .map_err(|error| error.to_string())
}

fn apply_whiteout_to_edgefs<S: BlockStorage>(
    fs: &mut EdgeFs<S>,
    whiteout: &OciWhiteout,
) -> Result<(), String> {
    match whiteout {
        OciWhiteout::RemovePath(path) => fs.remove_path(path),
        OciWhiteout::OpaqueDirectory(path) => fs.remove_children(path),
    }
    .map_err(|error| error.to_string())
}

fn device_id(entry: &TarEntry) -> Result<DeviceId, String> {
    Ok(DeviceId {
        major: entry
            .dev_major
            .ok_or_else(|| format!("missing device major for {}", entry.path))?,
        minor: entry
            .dev_minor
            .ok_or_else(|| format!("missing device minor for {}", entry.path))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_apply::apply_bare_image_layer_blobs;
    use crate::tar_layer::{OciWhiteout, TarEntry};
    use crate::test_support::{
        bare_image_plan_for_layers as plan_for_layers, layer_descriptor as descriptor, tar,
        tar_entry, TestDigest,
    };
    use edgerun_storage::InMemoryBlockDevice;

    const KEY: [u8; 32] = [0x45; 32];

    fn entry(path: &str, kind: TarEntryKind) -> TarEntry {
        TarEntry {
            path: path.into(),
            kind,
            size: 0,
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            mtime: 123,
            dev_major: None,
            dev_minor: None,
            link_name: None,
            whiteout: None,
        }
    }

    #[test]
    fn applies_oci_entries_to_encrypted_edgefs() {
        let device = InMemoryBlockDevice::new(512, 256);
        let fs = EdgeFs::format_with_id(device, KEY, [0x21; 16]).unwrap();
        let mut sink = EdgeFsLayerSink::new(fs);

        sink.apply_entry(&entry("etc/hostname", TarEntryKind::Regular), b"edge")
            .unwrap();
        let mut symlink = entry("bin/sh", TarEntryKind::Symlink);
        symlink.link_name = Some("/bin/busybox".into());
        sink.apply_entry(&symlink, &[]).unwrap();

        let fs = sink.into_filesystem();
        assert_eq!(
            fs.read_file("etc/hostname").unwrap(),
            Some(b"edge".to_vec())
        );
        let hostname = fs
            .list()
            .into_iter()
            .find(|entry| entry.path == "etc/hostname")
            .unwrap();
        assert_eq!(hostname.meta.mtime, 123);
        assert_eq!(fs.read_link("bin/sh").unwrap(), Some("/bin/busybox".into()));
    }

    #[test]
    fn applies_hardlinks_as_readable_edgefs_files() {
        let device = InMemoryBlockDevice::new(512, 256);
        let fs = EdgeFs::format_with_id(device, KEY, [0x26; 16]).unwrap();
        let mut sink = EdgeFsLayerSink::new(fs);

        sink.apply_entry(&entry("bin/busybox", TarEntryKind::Regular), b"binary")
            .unwrap();
        let mut hardlink = entry("bin/sh", TarEntryKind::Hardlink);
        hardlink.link_name = Some("bin/busybox".into());
        sink.apply_entry(&hardlink, &[]).unwrap();

        let fs = sink.into_filesystem();
        assert_eq!(fs.read_file("bin/sh").unwrap(), Some(b"binary".to_vec()));
        assert_eq!(fs.file_len("bin/sh").unwrap(), Some(6));
        let mut buf = [0u8; 3];
        assert_eq!(fs.read_file_range("bin/sh", 2, &mut buf).unwrap(), Some(3));
        assert_eq!(&buf, b"nar");
        assert_eq!(OciRootfs::file_len(&fs, "/bin/sh").unwrap(), Some(6));
        assert_eq!(
            OciRootfs::read_file_range(&fs, "/bin/sh", 1, &mut buf).unwrap(),
            Some(3)
        );
        assert_eq!(&buf, b"ina");

        let shell = fs.entry("bin/sh").unwrap().unwrap();
        assert_eq!(shell.kind, EntryKind::Hardlink);
        assert_eq!(shell.link_target, Some("bin/busybox"));
        let shell = OciRootfs::entry(&fs, "/bin/sh").unwrap().unwrap();
        assert_eq!(shell.kind, OciRootfsEntryKind::Hardlink);
        assert_eq!(shell.link_name, Some("bin/busybox".into()));

        let bin_children = OciRootfs::children(&fs, "/bin").unwrap();
        assert_eq!(
            bin_children
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["bin/busybox", "bin/sh"]
        );

        let under_bin = OciRootfs::entries_under(&fs, "/bin").unwrap();
        assert_eq!(under_bin.len(), 2);
    }

    #[test]
    fn resolves_executables_in_edgefs_rootfs() {
        let device = InMemoryBlockDevice::new(512, 256);
        let fs = EdgeFs::format_with_id(device, KEY, [0x27; 16]).unwrap();
        let mut sink = EdgeFsLayerSink::new(fs);

        let mut app = entry("usr/bin/app", TarEntryKind::Regular);
        app.mode = 0o755;
        sink.apply_entry(&app, b"app!").unwrap();

        let mut shell = entry("bin/sh", TarEntryKind::Symlink);
        shell.mode = 0o777;
        shell.link_name = Some("../usr/bin/app".into());
        sink.apply_entry(&shell, &[]).unwrap();

        let fs = sink.into_filesystem();
        let executable = crate::rootfs_access::resolve_executable_path(
            &fs,
            "app",
            &["PATH=/usr/bin:/bin".into()],
            "/",
        )
        .unwrap()
        .unwrap();
        assert_eq!(executable.path, "usr/bin/app");

        let executable = crate::rootfs_access::resolve_executable_path(&fs, "/bin/sh", &[], "/")
            .unwrap()
            .unwrap();
        assert_eq!(executable.path, "usr/bin/app");

        let plan = crate::rootfs_access::build_launch_plan(
            &fs,
            &["app".into(), "--help".into()],
            &["PATH=/usr/bin".into()],
            "/",
        )
        .unwrap()
        .unwrap();
        assert_eq!(plan.executable.path, "usr/bin/app");
        assert_eq!(plan.argv, vec!["app", "--help"]);
        assert_eq!(plan.cwd, "/");
    }

    #[test]
    fn applies_whiteouts_to_edgefs() {
        let device = InMemoryBlockDevice::new(512, 256);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x22; 16]).unwrap();
        fs.write_file("var/cache/old", b"old", FileMeta::default())
            .unwrap();
        fs.write_file("var/lib/keep", b"keep", FileMeta::default())
            .unwrap();
        let mut sink = EdgeFsLayerSink::new(fs);

        let mut opaque = entry("var/cache/.wh..wh..opq", TarEntryKind::Regular);
        opaque.whiteout = Some(OciWhiteout::OpaqueDirectory("var/cache".into()));
        sink.apply_entry(&opaque, &[]).unwrap();

        let fs = sink.into_filesystem();
        assert_eq!(fs.read_file("var/cache/old").unwrap(), None);
        assert_eq!(
            fs.read_file("var/lib/keep").unwrap(),
            Some(b"keep".to_vec())
        );
    }

    #[test]
    fn preserves_device_ids_in_edgefs() {
        let device = InMemoryBlockDevice::new(512, 256);
        let fs = EdgeFs::format_with_id(device, KEY, [0x23; 16]).unwrap();
        let mut sink = EdgeFsLayerSink::new(fs);
        let mut entry = entry("dev/null", TarEntryKind::Character);
        entry.dev_major = Some(1);
        entry.dev_minor = Some(3);

        sink.apply_entry(&entry, &[]).unwrap();

        let fs = sink.into_filesystem();
        assert_eq!(
            fs.read_device("dev/null").unwrap(),
            Some(DeviceId { major: 1, minor: 3 })
        );
    }

    #[test]
    fn applies_image_layers_directly_to_edgefs() {
        let lower = tar(vec![tar_entry("etc/hostname", b'0', b"old")]);
        let upper = tar(vec![
            tar_entry("etc/.wh.hostname", b'0', b""),
            tar_entry("etc/hostname", b'0', b"edge"),
        ]);
        let plan = plan_for_layers(vec![descriptor(&lower), descriptor(&upper)]);
        let device = InMemoryBlockDevice::new(512, 256);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x24; 16]).unwrap();

        apply_bare_image_layer_blobs(
            &plan,
            [lower.as_slice(), upper.as_slice()],
            |_| TestDigest::default(),
            &mut fs,
        )
        .unwrap();

        assert_eq!(
            fs.read_file("etc/hostname").unwrap(),
            Some(b"edge".to_vec())
        );
    }

    #[test]
    fn compacts_edgefs_after_image_layers() {
        let lower = tar(vec![
            tar_entry("etc/hostname", b'0', b"old"),
            tar_entry("var/cache/deleted", b'0', b"old"),
        ]);
        let upper = tar(vec![
            tar_entry("etc/hostname", b'0', b"edge"),
            tar_entry("var/cache/.wh.deleted", b'0', b""),
        ]);
        let plan = plan_for_layers(vec![descriptor(&lower), descriptor(&upper)]);
        let device = InMemoryBlockDevice::new(512, 256);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x25; 16]).unwrap();

        apply_bare_image_layer_blobs(
            &plan,
            [lower.as_slice(), upper.as_slice()],
            |_| TestDigest::default(),
            &mut fs,
        )
        .unwrap();
        let before = fs.used_bytes();

        fs.compact().unwrap();

        assert!(fs.used_bytes() < before);
        assert_eq!(
            fs.read_file("etc/hostname").unwrap(),
            Some(b"edge".to_vec())
        );
        assert_eq!(fs.read_file("var/cache/deleted").unwrap(), None);
        let reopened = EdgeFs::open(fs.into_device(), KEY).unwrap();
        assert_eq!(
            reopened.read_file("etc/hostname").unwrap(),
            Some(b"edge".to_vec())
        );
        assert_eq!(reopened.read_file("var/cache/deleted").unwrap(), None);
    }
}
