//! EdgeFS-backed no_std OCI layer sink.

use crate::oci_path::normalize_layer_path;
use crate::prelude::*;
use crate::rootfs_access::{
    OciDeviceId, OciRootfs, OciRootfsEntry, OciRootfsEntryKind, OciRootfsError,
};
use crate::tar_layer::{OciWhiteout, TarEntry, TarEntryKind, TarLayerSink};
use crate::util::StringResultExt;
use edgerun_storage::{BlockStorage, DeviceId, EdgeFs, EdgeFsError, EntryKind, EntryRef, FileMeta};

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

fn edgefs_error(error: EdgeFsError) -> OciRootfsError {
    match error {
        EdgeFsError::InvalidPath(path) => OciRootfsError::InvalidPath(path),
        EdgeFsError::HardlinkLoop(path) => OciRootfsError::LinkLoop(path),
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
    .string_err()
}

fn apply_whiteout_to_edgefs<S: BlockStorage>(
    fs: &mut EdgeFs<S>,
    whiteout: &OciWhiteout,
) -> Result<(), String> {
    match whiteout {
        OciWhiteout::RemovePath(path) => fs.remove_path(path),
        OciWhiteout::OpaqueDirectory(path) => fs.remove_children(path),
    }
    .string_err()
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
