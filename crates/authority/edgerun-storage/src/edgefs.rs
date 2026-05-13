//! EdgeFS: encrypted, append-first block filesystem for Edgerun.
//!
//! This initial implementation deliberately keeps durable authority in an
//! encrypted append log. Public block bytes contain only filesystem geometry,
//! record sizes, nonces, and checkpoint cursor state. Paths, metadata, and file
//! contents are encrypted as AEAD payloads and indexes are rebuilt by scanning
//! the log.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use crate::BlockStorage;
use edgerun_crypto::{Aead, AesGcmCipher, KeyInit, Nonce};
use edgerun_protocols::wire as edgerun_wire;
use edgerun_protocols::wire::{EdgeFsDeviceId, EdgeFsFileMeta, EdgeFsRecordPayload};

const SUPER_MAGIC: &[u8; 8] = b"EDGEFS01";
const RECORD_MAGIC: &[u8; 8] = b"EFRCD001";
const VERSION: u16 = 1;
const SUPERBLOCK_A: u64 = 0;
const SUPERBLOCK_B: u64 = 1;
const DATA_START_SECTOR: u64 = 2;
const SUPERBLOCK_MIN_LEN: usize = 64;
const RECORD_HEADER_LEN: usize = 48;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const AEAD_TAG_LEN: usize = 16;
const DEFAULT_MODE_FILE: u32 = 0o644;
const DEFAULT_MODE_DIR: u32 = 0o755;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeFsError {
    InvalidKey,
    InvalidFormatId,
    InvalidGeometry(String),
    NotFormatted,
    CorruptSuperblock(String),
    CorruptRecord(String),
    Encryption,
    Decryption,
    Device(String),
    OutOfSpace,
    NotFound(String),
    InvalidPath(String),
    HardlinkLoop(String),
}

impl fmt::Display for EdgeFsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey => f.write_str("invalid EdgeFS encryption key"),
            Self::InvalidFormatId => f.write_str("invalid EdgeFS format identity"),
            Self::InvalidGeometry(e) => write!(f, "invalid block geometry: {e}"),
            Self::NotFormatted => f.write_str("device is not formatted as EdgeFS"),
            Self::CorruptSuperblock(e) => write!(f, "corrupt EdgeFS superblock: {e}"),
            Self::CorruptRecord(e) => write!(f, "corrupt EdgeFS record: {e}"),
            Self::Encryption => f.write_str("EdgeFS encryption failed"),
            Self::Decryption => f.write_str("EdgeFS decryption failed"),
            Self::Device(e) => write!(f, "block device error: {e}"),
            Self::OutOfSpace => f.write_str("EdgeFS device is full"),
            Self::NotFound(path) => write!(f, "path not found: {path}"),
            Self::InvalidPath(path) => write!(f, "invalid path: {path}"),
            Self::HardlinkLoop(path) => write!(f, "hardlink loop at path: {path}"),
        }
    }
}

impl core::error::Error for EdgeFsError {}

impl From<crate::StorageError> for EdgeFsError {
    fn from(value: crate::StorageError) -> Self {
        Self::Device(value.to_string())
    }
}

pub type Result<T> = core::result::Result<T, EdgeFsError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMeta {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: u64,
}

impl FileMeta {
    pub const fn file(mode: u32, uid: u32, gid: u32) -> Self {
        Self {
            mode,
            uid,
            gid,
            mtime: 0,
        }
    }

    pub const fn dir(mode: u32, uid: u32, gid: u32) -> Self {
        Self {
            mode,
            uid,
            gid,
            mtime: 0,
        }
    }

    pub const fn with_mtime(mut self, mtime: u64) -> Self {
        self.mtime = mtime;
        self
    }
}

impl Default for FileMeta {
    fn default() -> Self {
        Self {
            mode: DEFAULT_MODE_FILE,
            uid: 0,
            gid: 0,
            mtime: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Hardlink,
    Character,
    Block,
    Fifo,
}

impl EntryKind {
    fn to_u8(self) -> u8 {
        match self {
            Self::File => 1,
            Self::Directory => 2,
            Self::Symlink => 3,
            Self::Hardlink => 4,
            Self::Character => 5,
            Self::Block => 6,
            Self::Fifo => 7,
        }
    }

    fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::File),
            2 => Some(Self::Directory),
            3 => Some(Self::Symlink),
            4 => Some(Self::Hardlink),
            5 => Some(Self::Character),
            6 => Some(Self::Block),
            7 => Some(Self::Fifo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceId {
    pub major: u32,
    pub minor: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub path: String,
    pub kind: EntryKind,
    pub meta: FileMeta,
    pub len: usize,
    pub device: Option<DeviceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryRef<'a> {
    pub path: &'a str,
    pub kind: EntryKind,
    pub meta: FileMeta,
    pub data: &'a [u8],
    pub link_target: Option<&'a str>,
    pub device: Option<DeviceId>,
}

impl EntryRef<'_> {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[derive(Clone)]
struct IndexedEntry {
    kind: EntryKind,
    meta: FileMeta,
    data: Vec<u8>,
    link_target: Option<String>,
    device: Option<DeviceId>,
}

#[derive(Debug, Clone)]
struct Superblock {
    generation: u64,
    cursor: u64,
    sector_size: usize,
    sectors: u64,
    fs_id: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeFsInfo {
    pub generation: u64,
    pub cursor: u64,
    pub sector_size: usize,
    pub sectors: u64,
    pub fs_id: [u8; 16],
}

#[derive(Clone)]
struct KeyMaterial {
    record: [u8; KEY_LEN],
    nonce: [u8; KEY_LEN],
    superblock: [u8; KEY_LEN],
}

/// A mandatory-encryption block filesystem over a sector-addressable device.
pub struct EdgeFs<S: BlockStorage> {
    device: S,
    keys: KeyMaterial,
    fs_id: [u8; 16],
    sector_size: usize,
    sectors: u64,
    cursor: u64,
    generation: u64,
    entries: BTreeMap<String, IndexedEntry>,
}

impl<S: BlockStorage> EdgeFs<S> {
    /// Reformat an existing EdgeFS volume with a new nonce domain.
    ///
    /// This only works when the device already contains a superblock
    /// authenticated by `key`. For blank media, use [`Self::format_with_id`]
    /// with a unique nonzero filesystem ID from a hardware RNG or provisioning
    /// flow. EdgeFS does not invent randomness in no_std code.
    pub fn format(mut device: S, key: [u8; KEY_LEN]) -> Result<Self> {
        validate_key(&key)?;
        let sector_size = device.sector_size();
        let sectors = device.sectors();
        validate_geometry(sector_size, sectors)?;

        let fs_id = derive_next_fs_id(&mut device, &key, sector_size, sectors)?;
        Self::format_with_geometry(device, key, fs_id, sector_size, sectors)
    }

    /// Format blank media using a caller-provided filesystem identity.
    ///
    /// The `fs_id` is public in the superblock, but it defines the AEAD nonce
    /// domain for this volume. It must be unique for each format under the same
    /// encryption key. Passing all zeroes is rejected.
    pub fn format_with_id(mut device: S, key: [u8; KEY_LEN], fs_id: [u8; 16]) -> Result<Self> {
        validate_key(&key)?;
        validate_fs_id(&fs_id)?;
        let sector_size = device.sector_size();
        let sectors = device.sectors();
        validate_geometry(sector_size, sectors)?;
        Self::format_with_geometry(device, key, fs_id, sector_size, sectors)
    }

    fn format_with_geometry(
        mut device: S,
        key: [u8; KEY_LEN],
        fs_id: [u8; 16],
        sector_size: usize,
        sectors: u64,
    ) -> Result<Self> {
        validate_fs_id(&fs_id)?;
        let keys = derive_key_material(&key, &fs_id);
        let superblock = Superblock {
            generation: 1,
            cursor: DATA_START_SECTOR.saturating_mul(sector_size as u64),
            sector_size,
            sectors,
            fs_id,
        };
        write_superblocks(&mut device, &keys.superblock, &superblock)?;
        device.sync()?;

        Ok(Self {
            device,
            keys,
            fs_id,
            sector_size,
            sectors,
            cursor: superblock.cursor,
            generation: superblock.generation,
            entries: BTreeMap::new(),
        })
    }

    pub fn open(mut device: S, key: [u8; KEY_LEN]) -> Result<Self> {
        validate_key(&key)?;
        let sector_size = device.sector_size();
        let sectors = device.sectors();
        validate_geometry(sector_size, sectors)?;
        let superblock_key = derive_superblock_key(&key);
        let superblock = read_best_superblock(&mut device, &superblock_key, sector_size)?;
        if superblock.sector_size != sector_size || superblock.sectors != sectors {
            return Err(EdgeFsError::CorruptSuperblock(
                "stored geometry does not match device".into(),
            ));
        }

        let keys = derive_key_material(&key, &superblock.fs_id);
        let mut fs = Self {
            device,
            keys,
            fs_id: superblock.fs_id,
            sector_size,
            sectors,
            cursor: superblock.cursor,
            generation: superblock.generation,
            entries: BTreeMap::new(),
        };
        fs.rebuild_index()?;
        Ok(fs)
    }

    /// Probe an encrypted EdgeFS superblock using the supplied volume key.
    ///
    /// EdgeFS keeps the public superblock authenticated with a key-derived tag,
    /// so probing is intentionally key-aware. A wrong key returns a corrupt
    /// superblock error instead of a false mount.
    pub fn probe(mut device: S, key: [u8; KEY_LEN]) -> Result<EdgeFsInfo> {
        validate_key(&key)?;
        let sector_size = device.sector_size();
        let sectors = device.sectors();
        validate_geometry(sector_size, sectors)?;
        let superblock_key = derive_superblock_key(&key);
        let superblock = read_best_superblock(&mut device, &superblock_key, sector_size)?;
        if superblock.sector_size != sector_size || superblock.sectors != sectors {
            return Err(EdgeFsError::CorruptSuperblock(
                "stored geometry does not match device".into(),
            ));
        }
        Ok(superblock.info())
    }

    pub fn into_device(self) -> S {
        self.device
    }

    pub fn block_size(&self) -> usize {
        self.sector_size
    }

    /// Return the public filesystem identity used for nonce-domain separation.
    pub fn filesystem_id(&self) -> [u8; 16] {
        self.fs_id
    }

    pub fn used_bytes(&self) -> u64 {
        self.cursor
            .saturating_sub(DATA_START_SECTOR.saturating_mul(self.sector_size as u64))
    }

    pub fn available_bytes(&self) -> u64 {
        self.device_bytes().saturating_sub(self.cursor)
    }

    pub fn compacted_used_bytes(&self) -> Result<u64> {
        estimate_payloads_size(&self.current_payloads(), self.sector_size)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, path: &str) -> bool {
        normalize_path_checked(path, true)
            .map(|path| self.entries.contains_key(path.as_str()))
            .unwrap_or(false)
    }

    pub fn entry(&self, path: &str) -> Result<Option<EntryRef<'_>>> {
        let path = normalize_path_checked(path, true)?;
        Ok(self
            .entries
            .get_key_value(&path)
            .map(|(stored_path, entry)| entry_ref(stored_path.as_str(), entry)))
    }

    pub fn iter_entries(&self) -> impl Iterator<Item = EntryRef<'_>> {
        self.entries
            .iter()
            .map(|(path, entry)| entry_ref(path.as_str(), entry))
    }

    pub fn entries_under<'a>(
        &'a self,
        path: &str,
    ) -> Result<impl Iterator<Item = EntryRef<'a>> + 'a> {
        let path = normalize_path_checked(path, true)?;
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}/")
        };
        Ok(self.iter_entries().filter(move |entry| {
            if prefix.is_empty() {
                true
            } else {
                entry.path.starts_with(prefix.as_str())
            }
        }))
    }

    pub fn children<'a>(&'a self, path: &str) -> Result<impl Iterator<Item = EntryRef<'a>> + 'a> {
        let path = normalize_path_checked(path, true)?;
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{path}/")
        };
        Ok(self.iter_entries().filter(move |entry| {
            let relative = if prefix.is_empty() {
                entry.path
            } else if let Some(relative) = entry.path.strip_prefix(prefix.as_str()) {
                relative
            } else {
                return false;
            };
            !relative.is_empty() && !relative.contains('/')
        }))
    }

    pub fn mkdir_all(&mut self, path: &str, meta: FileMeta) -> Result<()> {
        let path = normalize_path_checked(path, true)?;
        if path.is_empty() {
            return Ok(());
        }

        let mut current = String::new();
        for part in path.split('/') {
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(part);
            if !self.entries.contains_key(&current) {
                self.append_entry(RecordPayload::Put {
                    path: current.clone(),
                    kind: EntryKind::Directory,
                    meta,
                    data: Vec::new(),
                    link_target: None,
                    device: None,
                })?;
            }
        }
        Ok(())
    }

    pub fn write_file(&mut self, path: &str, bytes: &[u8], meta: FileMeta) -> Result<()> {
        let path = normalize_non_empty_path(path)?;
        self.ensure_parent_dirs(&path, meta)?;
        self.append_entry(RecordPayload::Put {
            path,
            kind: EntryKind::File,
            meta,
            data: bytes.to_vec(),
            link_target: None,
            device: None,
        })
    }

    pub fn symlink(&mut self, path: &str, target: &str, meta: FileMeta) -> Result<()> {
        let path = normalize_non_empty_path(path)?;
        self.ensure_parent_dirs(&path, meta)?;
        self.append_entry(RecordPayload::Put {
            path,
            kind: EntryKind::Symlink,
            meta,
            data: Vec::new(),
            link_target: Some(target.into()),
            device: None,
        })
    }

    pub fn hardlink(&mut self, path: &str, target: &str, meta: FileMeta) -> Result<()> {
        let path = normalize_non_empty_path(path)?;
        let target = normalize_path_checked(target, false)?;
        self.ensure_parent_dirs(&path, meta)?;
        self.append_entry(RecordPayload::Put {
            path,
            kind: EntryKind::Hardlink,
            meta,
            data: Vec::new(),
            link_target: Some(target),
            device: None,
        })
    }

    pub fn create_node(&mut self, path: &str, kind: EntryKind, meta: FileMeta) -> Result<()> {
        if !matches!(
            kind,
            EntryKind::Directory | EntryKind::Character | EntryKind::Block | EntryKind::Fifo
        ) {
            return Err(EdgeFsError::InvalidPath(
                "create_node requires a directory, device, or fifo kind".into(),
            ));
        }
        if kind == EntryKind::Directory {
            return self.mkdir_all(path, meta);
        }

        let path = normalize_non_empty_path(path)?;
        self.ensure_parent_dirs(&path, meta)?;
        self.append_entry(RecordPayload::Put {
            path,
            kind,
            meta,
            data: Vec::new(),
            link_target: None,
            device: None,
        })
    }

    pub fn create_device_node(
        &mut self,
        path: &str,
        kind: EntryKind,
        meta: FileMeta,
        device: DeviceId,
    ) -> Result<()> {
        if !matches!(kind, EntryKind::Character | EntryKind::Block) {
            return Err(EdgeFsError::InvalidPath(
                "create_device_node requires a character or block kind".into(),
            ));
        }

        let path = normalize_non_empty_path(path)?;
        self.ensure_parent_dirs(&path, meta)?;
        self.append_entry(RecordPayload::Put {
            path,
            kind,
            meta,
            data: Vec::new(),
            link_target: None,
            device: Some(device),
        })
    }

    pub fn remove_path(&mut self, path: &str) -> Result<()> {
        let path = normalize_non_empty_path(path)?;
        self.append_entry(RecordPayload::Delete { path })
    }

    pub fn remove_children(&mut self, path: &str) -> Result<()> {
        let path = normalize_path_checked(path, true)?;
        self.append_entry(RecordPayload::DeleteChildren { path })
    }

    pub fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.read_file_ref(path)?.map(|bytes| bytes.to_vec()))
    }

    pub fn read_file_ref(&self, path: &str) -> Result<Option<&[u8]>> {
        let original = normalize_path_checked(path, true)?;
        let mut current = original.clone();

        for _ in 0..=self.entries.len() {
            let Some(entry) = self.entries.get(current.as_str()) else {
                return Ok(None);
            };

            match entry.kind {
                EntryKind::File => return Ok(Some(entry.data.as_slice())),
                EntryKind::Hardlink => {
                    let Some(target) = entry.link_target.as_deref() else {
                        return Ok(None);
                    };
                    current = normalize_path_checked(target, false)?;
                }
                _ => return Ok(None),
            }
        }

        Err(EdgeFsError::HardlinkLoop(original))
    }

    pub fn file_len(&self, path: &str) -> Result<Option<usize>> {
        Ok(self.read_file_ref(path)?.map(|bytes| bytes.len()))
    }

    pub fn read_file_range(
        &self,
        path: &str,
        offset: u64,
        out: &mut [u8],
    ) -> Result<Option<usize>> {
        let Some(bytes) = self.read_file_ref(path)? else {
            return Ok(None);
        };
        let offset = usize::try_from(offset).map_err(|_| EdgeFsError::OutOfSpace)?;
        if offset >= bytes.len() || out.is_empty() {
            return Ok(Some(0));
        }

        let count = core::cmp::min(out.len(), bytes.len() - offset);
        out[..count].copy_from_slice(&bytes[offset..offset + count]);
        Ok(Some(count))
    }

    pub fn read_link(&self, path: &str) -> Result<Option<String>> {
        Ok(self.read_link_ref(path)?.map(String::from))
    }

    pub fn read_link_ref(&self, path: &str) -> Result<Option<&str>> {
        Ok(self.entry(path)?.and_then(|entry| {
            matches!(entry.kind, EntryKind::Symlink | EntryKind::Hardlink)
                .then_some(entry.link_target)
                .flatten()
        }))
    }

    pub fn read_device(&self, path: &str) -> Result<Option<DeviceId>> {
        Ok(self.entry(path)?.and_then(|entry| entry.device))
    }

    pub fn list(&self) -> Vec<DirEntry> {
        self.iter_entries()
            .map(|entry| DirEntry {
                path: entry.path.into(),
                kind: entry.kind,
                meta: entry.meta,
                len: entry.len(),
                device: entry.device,
            })
            .collect()
    }

    /// Rewrite the current tree into a new compact encrypted log.
    ///
    /// This reclaims append-log space from overwritten files, deletes, and OCI
    /// whiteouts. It is intended for controlled points after image layer
    /// application. The current implementation rewrites in place, so callers
    /// that need crash-atomic compaction should compact into a separate volume
    /// with [`Self::compact_into`] and switch over after the new volume opens.
    pub fn compact(&mut self) -> Result<()> {
        let payloads = self.current_payloads();

        let old_cursor = self.cursor;
        self.cursor = DATA_START_SECTOR.saturating_mul(self.sector_size as u64);
        self.entries.clear();
        self.generation = self.generation.saturating_add(1);

        for payload in payloads {
            self.append_entry(payload)?;
        }

        self.zero_stale_log_tail(self.cursor, old_cursor)?;
        let superblock = Superblock {
            generation: self.generation,
            cursor: self.cursor,
            sector_size: self.sector_size,
            sectors: self.sectors,
            fs_id: self.fs_id,
        };
        write_superblocks(&mut self.device, &self.keys.superblock, &superblock)?;
        self.device.sync()?;
        Ok(())
    }

    /// Build a compact encrypted copy of this filesystem on another device.
    ///
    /// The source volume is not modified. Use this when the caller has a spare
    /// partition/device and wants a safer swap-based compaction flow.
    pub fn compact_into<T: BlockStorage>(
        &self,
        target: T,
        key: [u8; KEY_LEN],
        fs_id: [u8; 16],
    ) -> Result<EdgeFs<T>> {
        validate_key(&key)?;
        validate_fs_id(&fs_id)?;
        let target_sector_size = target.sector_size();
        let target_sectors = target.sectors();
        validate_geometry(target_sector_size, target_sectors)?;
        let payloads = self.current_payloads();
        let required = estimate_payloads_size(&payloads, target_sector_size)?;
        let available = target_sectors
            .saturating_sub(DATA_START_SECTOR)
            .saturating_mul(target_sector_size as u64);
        if required > available {
            return Err(EdgeFsError::OutOfSpace);
        }

        let mut compacted = EdgeFs::format_with_id(target, key, fs_id)?;
        for payload in payloads {
            compacted.append_entry(payload)?;
        }
        Ok(compacted)
    }

    fn ensure_parent_dirs(&mut self, path: &str, meta: FileMeta) -> Result<()> {
        if let Some((parent, _)) = path.rsplit_once('/') {
            self.mkdir_all(parent, FileMeta::dir(DEFAULT_MODE_DIR, meta.uid, meta.gid))?;
        }
        Ok(())
    }

    fn current_payloads(&self) -> Vec<RecordPayload> {
        self.entries
            .iter()
            .map(|(path, entry)| RecordPayload::Put {
                path: path.clone(),
                kind: entry.kind,
                meta: entry.meta,
                data: entry.data.clone(),
                link_target: entry.link_target.clone(),
                device: entry.device,
            })
            .collect()
    }

    fn append_entry(&mut self, payload: RecordPayload) -> Result<()> {
        let offset = self.cursor;
        let plaintext = encode_payload(&payload)?;
        let nonce = self.nonce_for_offset(offset);
        let aad = self.record_aad(offset);
        let ciphertext = encrypt_payload(&self.keys.record, &nonce, &aad, &plaintext)?;
        let record_len = RECORD_HEADER_LEN + ciphertext.len();
        let padded_len = round_up(record_len, self.sector_size)?;

        if offset.saturating_add(padded_len as u64) > self.device_bytes() {
            return Err(EdgeFsError::OutOfSpace);
        }

        let mut record = vec![0u8; padded_len];
        encode_record_header(
            &mut record[..RECORD_HEADER_LEN],
            &nonce,
            plaintext.len(),
            ciphertext.len(),
        )?;
        record[RECORD_HEADER_LEN..RECORD_HEADER_LEN + ciphertext.len()]
            .copy_from_slice(&ciphertext);
        self.write_bytes(offset, &record)?;

        self.apply_payload(payload);
        self.cursor = self.cursor.saturating_add(padded_len as u64);
        self.generation = self.generation.saturating_add(1);
        let superblock = Superblock {
            generation: self.generation,
            cursor: self.cursor,
            sector_size: self.sector_size,
            sectors: self.sectors,
            fs_id: self.fs_id,
        };
        write_superblocks(&mut self.device, &self.keys.superblock, &superblock)?;
        self.device.sync()?;
        Ok(())
    }

    fn rebuild_index(&mut self) -> Result<()> {
        self.entries.clear();
        let mut offset = DATA_START_SECTOR.saturating_mul(self.sector_size as u64);
        while offset < self.cursor {
            let header = self.read_record_header(offset)?;
            let total_len = round_up(RECORD_HEADER_LEN + header.ciphertext_len, self.sector_size)?;
            if offset.saturating_add(total_len as u64) > self.cursor {
                return Err(EdgeFsError::CorruptRecord(
                    "record extends past checkpoint cursor".into(),
                ));
            }

            let mut ciphertext = vec![0u8; header.ciphertext_len];
            self.read_bytes(offset + RECORD_HEADER_LEN as u64, &mut ciphertext)?;
            let aad = self.record_aad(offset);
            let plaintext = decrypt_payload(&self.keys.record, &header.nonce, &aad, &ciphertext)?;
            if plaintext.len() != header.plaintext_len {
                return Err(EdgeFsError::CorruptRecord(
                    "plaintext length mismatch".into(),
                ));
            }
            let payload = decode_payload(&plaintext)?;
            self.apply_payload(payload);
            offset = offset.saturating_add(total_len as u64);
        }
        Ok(())
    }

    fn apply_payload(&mut self, payload: RecordPayload) {
        match payload {
            RecordPayload::Put {
                path,
                kind,
                meta,
                data,
                link_target,
                device,
            } => {
                self.entries.insert(
                    path,
                    IndexedEntry {
                        kind,
                        meta,
                        data,
                        link_target,
                        device,
                    },
                );
            }
            RecordPayload::Delete { path } => {
                self.entries.remove(&path);
                let prefix = format!("{path}/");
                self.entries
                    .retain(|entry_path, _| !entry_path.starts_with(&prefix));
            }
            RecordPayload::DeleteChildren { path } => {
                let prefix = if path.is_empty() {
                    String::new()
                } else {
                    format!("{path}/")
                };
                self.entries.retain(|entry_path, _| {
                    if prefix.is_empty() {
                        false
                    } else {
                        !entry_path.starts_with(&prefix)
                    }
                });
            }
        }
    }

    fn nonce_for_offset(&self, offset: u64) -> [u8; NONCE_LEN] {
        let mut input = Vec::with_capacity(self.fs_id.len() + 8 + 13);
        input.extend_from_slice(&self.fs_id);
        input.extend_from_slice(&offset.to_le_bytes());
        input.extend_from_slice(b"edgefs:nonce");
        let hash = hmac_tag(&self.keys.nonce, &input);
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&hash[..NONCE_LEN]);
        nonce
    }

    fn record_aad(&self, offset: u64) -> Vec<u8> {
        let mut aad = Vec::with_capacity(8 + 16 + 8 + 2);
        aad.extend_from_slice(b"edgefs:r");
        aad.extend_from_slice(&self.fs_id);
        aad.extend_from_slice(&offset.to_le_bytes());
        aad.extend_from_slice(&VERSION.to_le_bytes());
        aad
    }

    fn read_record_header(&mut self, offset: u64) -> Result<RecordHeader> {
        let mut header = [0u8; RECORD_HEADER_LEN];
        self.read_bytes(offset, &mut header)?;
        decode_record_header(&header)
    }

    fn read_bytes(&mut self, offset: u64, out: &mut [u8]) -> Result<()> {
        transfer_bytes(&mut self.device, self.sector_size, offset, out, false)
    }

    fn write_bytes(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        let mut owned = data.to_vec();
        transfer_bytes(&mut self.device, self.sector_size, offset, &mut owned, true)
    }

    fn device_bytes(&self) -> u64 {
        self.sectors.saturating_mul(self.sector_size as u64)
    }

    fn zero_stale_log_tail(&mut self, start: u64, end: u64) -> Result<()> {
        if end <= start {
            return Ok(());
        }
        let zeroes = vec![0u8; self.sector_size];
        let first_sector = round_up(start as usize, self.sector_size)? / self.sector_size;
        let last_sector = end.div_ceil(self.sector_size as u64) as usize;
        for sector in first_sector..last_sector {
            self.device.write_sector(sector as u64, &zeroes)?;
        }
        Ok(())
    }
}

impl Superblock {
    fn info(&self) -> EdgeFsInfo {
        EdgeFsInfo {
            generation: self.generation,
            cursor: self.cursor,
            sector_size: self.sector_size,
            sectors: self.sectors,
            fs_id: self.fs_id,
        }
    }
}

#[derive(Debug, Clone)]
enum RecordPayload {
    Put {
        path: String,
        kind: EntryKind,
        meta: FileMeta,
        data: Vec<u8>,
        link_target: Option<String>,
        device: Option<DeviceId>,
    },
    Delete {
        path: String,
    },
    DeleteChildren {
        path: String,
    },
}

fn entry_ref<'a>(path: &'a str, entry: &'a IndexedEntry) -> EntryRef<'a> {
    EntryRef {
        path,
        kind: entry.kind,
        meta: entry.meta,
        data: entry.data.as_slice(),
        link_target: entry.link_target.as_deref(),
        device: entry.device,
    }
}

struct RecordHeader {
    nonce: [u8; NONCE_LEN],
    plaintext_len: usize,
    ciphertext_len: usize,
}

fn validate_key(key: &[u8; KEY_LEN]) -> Result<()> {
    if key.iter().all(|byte| *byte == 0) {
        return Err(EdgeFsError::InvalidKey);
    }
    Ok(())
}

fn validate_fs_id(fs_id: &[u8; 16]) -> Result<()> {
    if fs_id.iter().all(|byte| *byte == 0) {
        return Err(EdgeFsError::InvalidFormatId);
    }
    Ok(())
}

fn validate_geometry(sector_size: usize, sectors: u64) -> Result<()> {
    if sector_size < SUPERBLOCK_MIN_LEN || sector_size == 0 {
        return Err(EdgeFsError::InvalidGeometry(
            "sector size is too small".into(),
        ));
    }
    if sectors <= DATA_START_SECTOR {
        return Err(EdgeFsError::InvalidGeometry(
            "device has no data sectors".into(),
        ));
    }
    Ok(())
}

fn derive_key_material(master_key: &[u8; KEY_LEN], fs_id: &[u8; 16]) -> KeyMaterial {
    KeyMaterial {
        record: derive_labeled_key(master_key, b"edgefs:v1:record", Some(fs_id)),
        nonce: derive_labeled_key(master_key, b"edgefs:v1:nonce", Some(fs_id)),
        superblock: derive_superblock_key(master_key),
    }
}

fn derive_superblock_key(master_key: &[u8; KEY_LEN]) -> [u8; KEY_LEN] {
    derive_labeled_key(master_key, b"edgefs:v1:superblock", None)
}

fn derive_labeled_key(
    master_key: &[u8; KEY_LEN],
    label: &[u8],
    fs_id: Option<&[u8; 16]>,
) -> [u8; KEY_LEN] {
    let mut input = Vec::with_capacity(label.len() + fs_id.map(|id| id.len()).unwrap_or(0));
    input.extend_from_slice(label);
    if let Some(fs_id) = fs_id {
        input.extend_from_slice(fs_id);
    }
    hmac_tag(master_key, &input)
}

fn hmac_tag(key: &[u8], data: &[u8]) -> [u8; KEY_LEN] {
    let tag = edgerun_crypto::hmac_sha256(key, data);
    let mut out = [0u8; KEY_LEN];
    out.copy_from_slice(&tag[..KEY_LEN]);
    out
}

fn derive_next_fs_id<S: BlockStorage>(
    device: &mut S,
    key: &[u8; KEY_LEN],
    sector_size: usize,
    sectors: u64,
) -> Result<[u8; 16]> {
    let superblock_key = derive_superblock_key(key);
    let previous = read_best_superblock(device, &superblock_key, sector_size).ok();
    if previous.is_none() {
        return Err(EdgeFsError::InvalidFormatId);
    }
    let sample = read_format_sample(device, sector_size);
    let mut input = Vec::with_capacity(KEY_LEN + 8 + 8 + 64);
    input.extend_from_slice(key);
    input.extend_from_slice(&(sector_size as u64).to_le_bytes());
    input.extend_from_slice(&sectors.to_le_bytes());
    input.extend_from_slice(b"edgefs:id");
    if let Some(previous) = previous {
        input.extend_from_slice(&previous.generation.to_le_bytes());
        input.extend_from_slice(&previous.cursor.to_le_bytes());
        input.extend_from_slice(&previous.fs_id);
    } else {
        input.extend_from_slice(b"edgefs:first-format");
    }
    input.extend_from_slice(&sample);
    let hash = hmac_tag(key, &input);
    let mut fs_id = [0u8; 16];
    fs_id.copy_from_slice(&hash[..16]);
    Ok(fs_id)
}

fn read_format_sample<S: BlockStorage>(device: &mut S, sector_size: usize) -> Vec<u8> {
    let mut sample = Vec::new();
    for sector in [SUPERBLOCK_A, SUPERBLOCK_B, DATA_START_SECTOR] {
        let mut block = vec![0u8; sector_size];
        if device.read_sector(sector, &mut block).is_ok() {
            sample.extend_from_slice(&block);
        }
    }
    sample
}

fn write_superblocks<S: BlockStorage>(
    device: &mut S,
    superblock_key: &[u8; KEY_LEN],
    sb: &Superblock,
) -> Result<()> {
    let mut sector = vec![0u8; sb.sector_size];
    sector[..8].copy_from_slice(SUPER_MAGIC);
    put_u16(&mut sector, 8, VERSION);
    put_u16(&mut sector, 10, 64);
    put_u64(&mut sector, 12, sb.generation);
    put_u64(&mut sector, 20, sb.cursor);
    put_u32(&mut sector, 28, sb.sector_size as u32);
    put_u64(&mut sector, 32, sb.sectors);
    sector[40..56].copy_from_slice(&sb.fs_id);
    let checksum = superblock_tag(superblock_key, &sector[..56]);
    sector[56..64].copy_from_slice(&checksum[..8]);
    device.write_sector(SUPERBLOCK_A, &sector)?;
    device.write_sector(SUPERBLOCK_B, &sector)?;
    Ok(())
}

fn read_best_superblock<S: BlockStorage>(
    device: &mut S,
    superblock_key: &[u8; KEY_LEN],
    sector_size: usize,
) -> Result<Superblock> {
    let a = read_superblock_at(device, superblock_key, sector_size, SUPERBLOCK_A);
    let b = read_superblock_at(device, superblock_key, sector_size, SUPERBLOCK_B);
    match (a, b) {
        (Ok(left), Ok(right)) => Ok(if left.generation >= right.generation {
            left
        } else {
            right
        }),
        (Ok(sb), Err(_)) | (Err(_), Ok(sb)) => Ok(sb),
        (Err(left), Err(right))
            if !matches!(left, EdgeFsError::NotFormatted)
                || !matches!(right, EdgeFsError::NotFormatted) =>
        {
            Err(left)
        }
        (Err(_), Err(_)) => Err(EdgeFsError::NotFormatted),
    }
}

fn read_superblock_at<S: BlockStorage>(
    device: &mut S,
    superblock_key: &[u8; KEY_LEN],
    sector_size: usize,
    sector_index: u64,
) -> Result<Superblock> {
    let mut sector = vec![0u8; sector_size];
    device.read_sector(sector_index, &mut sector)?;
    if &sector[..8] != SUPER_MAGIC {
        return Err(EdgeFsError::NotFormatted);
    }
    if get_u16(&sector, 8)? != VERSION {
        return Err(EdgeFsError::CorruptSuperblock("unsupported version".into()));
    }
    let checksum = superblock_tag(superblock_key, &sector[..56]);
    if sector[56..64] != checksum[..8] {
        return Err(EdgeFsError::CorruptSuperblock("checksum mismatch".into()));
    }

    let stored_sector_size = get_u32(&sector, 28)? as usize;
    let sectors = get_u64(&sector, 32)?;
    let mut fs_id = [0u8; 16];
    fs_id.copy_from_slice(&sector[40..56]);
    Ok(Superblock {
        generation: get_u64(&sector, 12)?,
        cursor: get_u64(&sector, 20)?,
        sector_size: stored_sector_size,
        sectors,
        fs_id,
    })
}

fn superblock_tag(superblock_key: &[u8; KEY_LEN], header: &[u8]) -> [u8; 32] {
    hmac_tag(superblock_key, header)
}

fn estimate_payloads_size(payloads: &[RecordPayload], sector_size: usize) -> Result<u64> {
    let mut total = 0u64;
    for payload in payloads {
        let plaintext_len = encoded_payload_len(payload)?;
        let record_len = RECORD_HEADER_LEN
            .checked_add(plaintext_len)
            .and_then(|len| len.checked_add(AEAD_TAG_LEN))
            .ok_or_else(|| EdgeFsError::OutOfSpace)?;
        let padded = round_up(record_len, sector_size)?;
        total = total
            .checked_add(padded as u64)
            .ok_or_else(|| EdgeFsError::OutOfSpace)?;
    }
    Ok(total)
}

fn encoded_payload_len(payload: &RecordPayload) -> Result<usize> {
    match payload {
        RecordPayload::Put {
            path,
            data,
            link_target,
            ..
        } => {
            let target_len = link_target.as_ref().map(|target| target.len()).unwrap_or(0);
            1usize
                .checked_add(1)
                .and_then(|len| len.checked_add(1))
                .and_then(|len| len.checked_add(4 * 7))
                .and_then(|len| len.checked_add(8 * 2))
                .and_then(|len| len.checked_add(path.len()))
                .and_then(|len| len.checked_add(target_len))
                .and_then(|len| len.checked_add(data.len()))
                .ok_or_else(|| EdgeFsError::OutOfSpace)
        }
        RecordPayload::Delete { path } | RecordPayload::DeleteChildren { path } => 1usize
            .checked_add(4)
            .and_then(|len| len.checked_add(path.len()))
            .ok_or_else(|| EdgeFsError::OutOfSpace),
    }
}

fn encrypt_payload(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let cipher = AesGcmCipher::new_from_slice(key).map_err(|_| EdgeFsError::InvalidKey)?;
    cipher
        .encrypt(
            Nonce::from_slice(nonce),
            edgerun_crypto::aes_gcm::aead::Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| EdgeFsError::Encryption)
}

fn decrypt_payload(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    let cipher = AesGcmCipher::new_from_slice(key).map_err(|_| EdgeFsError::InvalidKey)?;
    cipher
        .decrypt(
            Nonce::from_slice(nonce),
            edgerun_crypto::aes_gcm::aead::Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| EdgeFsError::Decryption)
}

fn encode_record_header(
    out: &mut [u8],
    nonce: &[u8; NONCE_LEN],
    plaintext_len: usize,
    ciphertext_len: usize,
) -> Result<()> {
    if out.len() != RECORD_HEADER_LEN {
        return Err(EdgeFsError::CorruptRecord("invalid header buffer".into()));
    }
    out[..8].copy_from_slice(RECORD_MAGIC);
    put_u16(out, 8, VERSION);
    put_u16(out, 10, RECORD_HEADER_LEN as u16);
    out[12..24].copy_from_slice(nonce);
    put_u64(out, 24, plaintext_len as u64);
    put_u64(out, 32, ciphertext_len as u64);
    let checksum = edgerun_crypto::sha256(&out[..40]);
    out[40..48].copy_from_slice(&checksum[..8]);
    Ok(())
}

fn decode_record_header(input: &[u8]) -> Result<RecordHeader> {
    if input.len() != RECORD_HEADER_LEN {
        return Err(EdgeFsError::CorruptRecord("short record header".into()));
    }
    if &input[..8] != RECORD_MAGIC {
        return Err(EdgeFsError::CorruptRecord("bad record magic".into()));
    }
    if get_u16(input, 8)? != VERSION {
        return Err(EdgeFsError::CorruptRecord(
            "unsupported record version".into(),
        ));
    }
    if get_u16(input, 10)? as usize != RECORD_HEADER_LEN {
        return Err(EdgeFsError::CorruptRecord(
            "bad record header length".into(),
        ));
    }
    let checksum = edgerun_crypto::sha256(&input[..40]);
    if input[40..48] != checksum[..8] {
        return Err(EdgeFsError::CorruptRecord(
            "record header checksum mismatch".into(),
        ));
    }
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&input[12..24]);
    Ok(RecordHeader {
        nonce,
        plaintext_len: get_u64(input, 24)? as usize,
        ciphertext_len: get_u64(input, 32)? as usize,
    })
}

fn encode_payload(payload: &RecordPayload) -> Result<Vec<u8>> {
    let wire = match payload {
        RecordPayload::Put {
            path,
            kind,
            meta,
            data,
            link_target,
            device,
        } => {
            validate_normalized_path(path)?;
            EdgeFsRecordPayload::Put {
                path: path.clone(),
                kind: kind.to_u8(),
                meta: EdgeFsFileMeta {
                    mode: meta.mode,
                    uid: meta.uid,
                    gid: meta.gid,
                    mtime: meta.mtime,
                },
                data: data.clone(),
                link_target: link_target.clone(),
                device: device.map(|id| EdgeFsDeviceId {
                    major: id.major,
                    minor: id.minor,
                }),
            }
        }
        RecordPayload::Delete { path } => {
            validate_normalized_path(path)?;
            EdgeFsRecordPayload::Delete { path: path.clone() }
        }
        RecordPayload::DeleteChildren { path } => {
            validate_normalized_path(path)?;
            EdgeFsRecordPayload::DeleteChildren { path: path.clone() }
        }
    };
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&wire)
        .map(|bytes| bytes.into_vec())
        .map_err(|_| EdgeFsError::CorruptRecord("record payload rkyv encode failed".into()))
}

fn decode_payload(input: &[u8]) -> Result<RecordPayload> {
    let owned = input.to_vec();
    let wire = edgerun_wire::from_bytes::<EdgeFsRecordPayload, edgerun_wire::WireError>(&owned)
        .map_err(|_| EdgeFsError::CorruptRecord("record payload is not rkyv".into()))?;
    match wire {
        EdgeFsRecordPayload::Put {
            path,
            kind,
            meta,
            data,
            link_target,
            device,
        } => {
            validate_normalized_path(&path)?;
            let kind = EntryKind::from_u8(kind)
                .ok_or_else(|| EdgeFsError::CorruptRecord("unknown entry kind".into()))?;
            Ok(RecordPayload::Put {
                path,
                kind,
                meta: FileMeta {
                    mode: meta.mode,
                    uid: meta.uid,
                    gid: meta.gid,
                    mtime: meta.mtime,
                },
                data,
                link_target,
                device: device.map(|id| DeviceId {
                    major: id.major,
                    minor: id.minor,
                }),
            })
        }
        EdgeFsRecordPayload::Delete { path } => {
            validate_normalized_path(&path)?;
            Ok(RecordPayload::Delete { path })
        }
        EdgeFsRecordPayload::DeleteChildren { path } => {
            validate_normalized_path(&path)?;
            Ok(RecordPayload::DeleteChildren { path })
        }
    }
}

fn transfer_bytes<S: BlockStorage>(
    device: &mut S,
    sector_size: usize,
    offset: u64,
    buf: &mut [u8],
    write: bool,
) -> Result<()> {
    let mut remaining = buf.len();
    let mut pos = offset;
    let mut copied = 0usize;
    while remaining > 0 {
        let sector_index = pos / sector_size as u64;
        let sector_offset = (pos % sector_size as u64) as usize;
        let copy_len = core::cmp::min(remaining, sector_size - sector_offset);
        let range = copied..copied + copy_len;

        if write && sector_offset == 0 && copy_len == sector_size {
            device.write_sector(sector_index, &buf[range])?;
        } else {
            let mut sector = vec![0u8; sector_size];
            device.read_sector(sector_index, &mut sector)?;
            if write {
                sector[sector_offset..sector_offset + copy_len].copy_from_slice(&buf[range]);
                device.write_sector(sector_index, &sector)?;
            } else {
                buf[range].copy_from_slice(&sector[sector_offset..sector_offset + copy_len]);
            }
        }

        remaining -= copy_len;
        copied += copy_len;
        pos = pos.saturating_add(copy_len as u64);
    }
    Ok(())
}

fn normalize_path_checked(path: &str, allow_empty: bool) -> Result<String> {
    if path.contains('\0') {
        return Err(EdgeFsError::InvalidPath(path.into()));
    }

    let mut out = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if out.pop().is_none() {
                    return Err(EdgeFsError::InvalidPath(path.into()));
                }
            }
            other => out.push(other),
        }
    }
    let normalized = out.join("/");
    if !allow_empty && normalized.is_empty() {
        return Err(EdgeFsError::InvalidPath(path.into()));
    }
    Ok(normalized)
}

fn normalize_non_empty_path(path: &str) -> Result<String> {
    let path = normalize_path_checked(path, false)?;
    validate_normalized_path(&path)?;
    Ok(path)
}

fn validate_normalized_path(path: &str) -> Result<()> {
    if path.is_empty() || path.starts_with('/') || path.contains("/../") || path == ".." {
        return Err(EdgeFsError::InvalidPath(path.into()));
    }
    Ok(())
}

fn round_up(value: usize, unit: usize) -> Result<usize> {
    if unit == 0 {
        return Err(EdgeFsError::InvalidGeometry("zero block size".into()));
    }
    let rem = value % unit;
    Ok(if rem == 0 { value } else { value + unit - rem })
}

fn put_u16(buf: &mut [u8], offset: usize, value: u16) {
    buf[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(buf: &mut [u8], offset: usize, value: u64) {
    buf[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn get_u16(buf: &[u8], offset: usize) -> Result<u16> {
    let bytes: [u8; 2] = buf
        .get(offset..offset + 2)
        .ok_or_else(|| EdgeFsError::CorruptRecord("short u16".into()))?
        .try_into()
        .map_err(|_| EdgeFsError::CorruptRecord("bad u16".into()))?;
    Ok(u16::from_le_bytes(bytes))
}

fn get_u32(buf: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = buf
        .get(offset..offset + 4)
        .ok_or_else(|| EdgeFsError::CorruptRecord("short u32".into()))?
        .try_into()
        .map_err(|_| EdgeFsError::CorruptRecord("bad u32".into()))?;
    Ok(u32::from_le_bytes(bytes))
}

fn get_u64(buf: &[u8], offset: usize) -> Result<u64> {
    let bytes: [u8; 8] = buf
        .get(offset..offset + 8)
        .ok_or_else(|| EdgeFsError::CorruptRecord("short u64".into()))?
        .try_into()
        .map_err(|_| EdgeFsError::CorruptRecord("bad u64".into()))?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryBlockDevice;

    const KEY: [u8; 32] = [0x42; 32];

    fn raw_device_bytes(mut device: InMemoryBlockDevice) -> Vec<u8> {
        let sector_size = device.sector_size();
        let mut out = Vec::new();
        for sector in 0..device.sectors() {
            let mut buf = vec![0u8; sector_size];
            device.read_sector(sector, &mut buf).unwrap();
            out.extend_from_slice(&buf);
        }
        out
    }

    fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|candidate| candidate == needle)
    }

    #[test]
    fn rejects_zero_key() {
        let device = InMemoryBlockDevice::new(512, 64);
        assert!(matches!(
            EdgeFs::format(device, [0u8; 32]),
            Err(EdgeFsError::InvalidKey)
        ));
    }

    #[test]
    fn blank_format_requires_explicit_identity() {
        let device = InMemoryBlockDevice::new(512, 64);
        assert!(matches!(
            EdgeFs::format(device, KEY),
            Err(EdgeFsError::InvalidFormatId)
        ));
    }

    #[test]
    fn rejects_zero_format_id() {
        let device = InMemoryBlockDevice::new(512, 64);
        assert!(matches!(
            EdgeFs::format_with_id(device, KEY, [0u8; 16]),
            Err(EdgeFsError::InvalidFormatId)
        ));
    }

    #[test]
    fn format_with_id_uses_caller_identity() {
        let device = InMemoryBlockDevice::new(512, 64);
        let fs_id = [0x99; 16];
        let fs = EdgeFs::format_with_id(device, KEY, fs_id).unwrap();
        assert_eq!(fs.filesystem_id(), fs_id);
    }

    #[test]
    fn probe_reports_authenticated_edgefs_superblock() {
        let device = InMemoryBlockDevice::new(512, 64);
        let fs_id = [0x45; 16];
        let fs = EdgeFs::format_with_id(device, KEY, fs_id).unwrap();
        let info = EdgeFs::probe(fs.into_device(), KEY).unwrap();
        assert_eq!(info.fs_id, fs_id);
        assert_eq!(info.sector_size, 512);
        assert_eq!(info.sectors, 64);
        assert_eq!(info.generation, 1);
    }

    #[test]
    fn reformat_changes_filesystem_identity() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x11; 16]).unwrap();
        fs.write_file("secret.txt", b"secret", FileMeta::default())
            .unwrap();
        let first_id = fs.filesystem_id();

        let reformatted = EdgeFs::format(fs.into_device(), KEY).unwrap();

        assert_ne!(reformatted.filesystem_id(), first_id);
        assert_eq!(reformatted.read_file("secret.txt").unwrap(), None);
    }

    #[test]
    fn writes_reads_and_reopens_encrypted_files() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x12; 16]).unwrap();

        fs.write_file(
            "/etc/hostname",
            b"edge-node-01",
            FileMeta::file(0o644, 1000, 1000),
        )
        .unwrap();
        fs.symlink("bin/sh", "/bin/busybox", FileMeta::file(0o777, 0, 0))
            .unwrap();
        assert_eq!(
            fs.read_file("etc/hostname").unwrap(),
            Some(b"edge-node-01".to_vec())
        );
        assert_eq!(
            fs.read_file_ref("etc/hostname").unwrap(),
            Some(b"edge-node-01".as_slice())
        );
        let hostname = fs.entry("etc/hostname").unwrap().unwrap();
        assert_eq!(hostname.path, "etc/hostname");
        assert_eq!(hostname.kind, EntryKind::File);
        assert_eq!(hostname.meta.uid, 1000);
        assert_eq!(hostname.data, b"edge-node-01");
        assert_eq!(fs.read_link("bin/sh").unwrap(), Some("/bin/busybox".into()));
        assert_eq!(fs.read_link_ref("bin/sh").unwrap(), Some("/bin/busybox"));
        let shell = fs.entry("bin/sh").unwrap().unwrap();
        assert_eq!(shell.kind, EntryKind::Symlink);
        assert_eq!(shell.link_target, Some("/bin/busybox"));
        let all_paths = fs
            .iter_entries()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        assert!(all_paths.contains(&"etc/hostname"));
        assert!(all_paths.contains(&"bin/sh"));

        let device = fs.into_device();
        let raw = raw_device_bytes(device.clone());
        assert!(!contains_subslice(&raw, b"edge-node-01"));
        assert!(!contains_subslice(&raw, b"etc/hostname"));
        assert!(!contains_subslice(&raw, b"/bin/busybox"));

        let reopened = EdgeFs::open(device, KEY).unwrap();
        assert_eq!(
            reopened.read_file("etc/hostname").unwrap(),
            Some(b"edge-node-01".to_vec())
        );
        assert_eq!(
            reopened.read_link("bin/sh").unwrap(),
            Some("/bin/busybox".into())
        );
    }

    #[test]
    fn hardlinks_read_target_file_bytes() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x1d; 16]).unwrap();

        fs.write_file("bin/busybox", b"ELF-ish", FileMeta::file(0o755, 0, 0))
            .unwrap();
        fs.hardlink("bin/sh", "bin/busybox", FileMeta::file(0o755, 0, 0))
            .unwrap();

        assert_eq!(
            fs.read_file_ref("bin/sh").unwrap(),
            Some(b"ELF-ish".as_slice())
        );
        assert_eq!(fs.read_file("bin/sh").unwrap(), Some(b"ELF-ish".to_vec()));

        let shell = fs.entry("bin/sh").unwrap().unwrap();
        assert_eq!(shell.kind, EntryKind::Hardlink);
        assert_eq!(shell.link_target, Some("bin/busybox"));

        let reopened = EdgeFs::open(fs.into_device(), KEY).unwrap();
        assert_eq!(
            reopened.read_file("bin/sh").unwrap(),
            Some(b"ELF-ish".to_vec())
        );
    }

    #[test]
    fn reads_file_ranges_without_allocating_full_file() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x2a; 16]).unwrap();

        fs.write_file("bin/app", b"0123456789abcdef", FileMeta::file(0o755, 0, 0))
            .unwrap();
        fs.hardlink("bin/run", "bin/app", FileMeta::file(0o755, 0, 0))
            .unwrap();

        assert_eq!(fs.file_len("bin/app").unwrap(), Some(16));
        assert_eq!(fs.file_len("bin/run").unwrap(), Some(16));

        let mut buf = [0u8; 5];
        assert_eq!(fs.read_file_range("bin/run", 4, &mut buf).unwrap(), Some(5));
        assert_eq!(&buf, b"45678");
        assert_eq!(
            fs.read_file_range("bin/run", 16, &mut buf).unwrap(),
            Some(0)
        );
        assert_eq!(fs.read_file_range("missing", 0, &mut buf).unwrap(), None);
    }

    #[test]
    fn hardlink_loops_are_reported() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x1e; 16]).unwrap();

        fs.hardlink("a", "b", FileMeta::file(0o644, 0, 0)).unwrap();
        fs.hardlink("b", "a", FileMeta::file(0o644, 0, 0)).unwrap();

        assert!(matches!(
            fs.read_file_ref("a"),
            Err(EdgeFsError::HardlinkLoop(path)) if path == "a"
        ));
    }

    #[test]
    fn rejects_paths_that_escape_root() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x1f; 16]).unwrap();
        fs.write_file("etc/hostname", b"edge", FileMeta::default())
            .unwrap();

        assert!(matches!(
            fs.write_file("../etc/passwd", b"bad", FileMeta::default()),
            Err(EdgeFsError::InvalidPath(_))
        ));
        assert!(matches!(
            fs.read_file("../etc/hostname"),
            Err(EdgeFsError::InvalidPath(_))
        ));
        assert!(matches!(
            fs.children("../../etc"),
            Err(EdgeFsError::InvalidPath(_))
        ));
        assert!(!fs.contains("../etc/hostname"));
        assert_eq!(
            fs.read_file("etc/../etc/hostname").unwrap(),
            Some(b"edge".to_vec())
        );
    }

    #[test]
    fn rejects_hardlink_targets_that_escape_root() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x20; 16]).unwrap();

        assert!(matches!(
            fs.hardlink("bin/sh", "../bin/busybox", FileMeta::file(0o755, 0, 0)),
            Err(EdgeFsError::InvalidPath(_))
        ));
    }

    #[test]
    fn wrong_key_cannot_rebuild_index() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x13; 16]).unwrap();
        fs.write_file("secret.txt", b"super-secret", FileMeta::default())
            .unwrap();
        let device = fs.into_device();

        match EdgeFs::open(device, [0x24; 32]) {
            Ok(_) => panic!("wrong key unexpectedly opened EdgeFS"),
            Err(_) => {}
        };
    }

    #[test]
    fn rejects_tampered_superblock_checkpoint() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x14; 16]).unwrap();
        fs.write_file("secret.txt", b"super-secret", FileMeta::default())
            .unwrap();
        let mut device = fs.into_device();

        for sector_index in [SUPERBLOCK_A, SUPERBLOCK_B] {
            let mut sector = vec![0u8; device.sector_size()];
            device.read_sector(sector_index, &mut sector).unwrap();
            sector[20] ^= 0x40;
            device.write_sector(sector_index, &sector).unwrap();
        }

        assert!(EdgeFs::open(device, KEY).is_err());
    }

    #[test]
    fn delete_and_opaque_directory_replay() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x15; 16]).unwrap();
        fs.write_file("var/cache/old", b"old", FileMeta::default())
            .unwrap();
        fs.write_file("var/lib/keep", b"keep", FileMeta::default())
            .unwrap();
        fs.remove_children("var/cache").unwrap();
        fs.write_file("var/cache/new", b"new", FileMeta::default())
            .unwrap();
        fs.remove_path("var/lib/keep").unwrap();

        let reopened = EdgeFs::open(fs.into_device(), KEY).unwrap();
        assert_eq!(reopened.read_file("var/cache/old").unwrap(), None);
        assert_eq!(
            reopened.read_file("var/cache/new").unwrap(),
            Some(b"new".to_vec())
        );
        assert_eq!(reopened.read_file("var/lib/keep").unwrap(), None);
    }

    #[test]
    fn iterates_entries_under_prefix_without_allocating_list() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x1b; 16]).unwrap();
        fs.write_file("var/cache/a", b"a", FileMeta::default())
            .unwrap();
        fs.write_file("var/cache/nested/b", b"b", FileMeta::default())
            .unwrap();
        fs.write_file("var/lib/c", b"c", FileMeta::default())
            .unwrap();

        let cache_paths = fs
            .entries_under("var/cache")
            .unwrap()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();

        assert!(cache_paths.contains(&"var/cache/a"));
        assert!(cache_paths.contains(&"var/cache/nested"));
        assert!(cache_paths.contains(&"var/cache/nested/b"));
        assert!(!cache_paths.contains(&"var/lib/c"));
    }

    #[test]
    fn iterates_direct_children_without_allocating_list() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x1c; 16]).unwrap();
        fs.write_file("var/cache/a", b"a", FileMeta::default())
            .unwrap();
        fs.write_file("var/cache/nested/b", b"b", FileMeta::default())
            .unwrap();
        fs.write_file("var/lib/c", b"c", FileMeta::default())
            .unwrap();

        let root_children = fs
            .children("")
            .unwrap()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        assert_eq!(root_children, vec!["var"]);

        let cache_children = fs
            .children("var/cache")
            .unwrap()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();

        assert!(cache_children.contains(&"var/cache/a"));
        assert!(cache_children.contains(&"var/cache/nested"));
        assert!(!cache_children.contains(&"var/cache/nested/b"));
        assert!(!cache_children.contains(&"var/lib/c"));
    }

    #[test]
    fn compacts_log_to_current_tree() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(device, KEY, [0x16; 16]).unwrap();
        fs.write_file("etc/hostname", b"old", FileMeta::default())
            .unwrap();
        fs.write_file("etc/hostname", b"edge-node-01", FileMeta::default())
            .unwrap();
        fs.write_file("var/cache/deleted", b"delete-me", FileMeta::default())
            .unwrap();
        fs.remove_path("var/cache/deleted").unwrap();
        let before = fs.used_bytes();

        fs.compact().unwrap();

        assert!(fs.used_bytes() < before);
        assert_eq!(
            fs.read_file("etc/hostname").unwrap(),
            Some(b"edge-node-01".to_vec())
        );
        assert_eq!(fs.read_file("var/cache/deleted").unwrap(), None);

        let reopened = EdgeFs::open(fs.into_device(), KEY).unwrap();
        assert_eq!(
            reopened.read_file("etc/hostname").unwrap(),
            Some(b"edge-node-01".to_vec())
        );
        assert_eq!(reopened.read_file("var/cache/deleted").unwrap(), None);
    }

    #[test]
    fn compact_into_builds_reopenable_copy_without_touching_source() {
        let source_device = InMemoryBlockDevice::new(512, 128);
        let target_device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format_with_id(source_device, KEY, [0x17; 16]).unwrap();
        fs.write_file("etc/hostname", b"old", FileMeta::default())
            .unwrap();
        fs.write_file("etc/hostname", b"edge-node-01", FileMeta::default())
            .unwrap();
        fs.write_file("var/cache/deleted", b"delete-me", FileMeta::default())
            .unwrap();
        fs.remove_path("var/cache/deleted").unwrap();
        let source_used = fs.used_bytes();

        let compacted = fs.compact_into(target_device, KEY, [0x18; 16]).unwrap();

        assert_eq!(fs.used_bytes(), source_used);
        assert_eq!(compacted.used_bytes(), fs.compacted_used_bytes().unwrap());
        assert!(compacted.used_bytes() < source_used);
        assert_eq!(
            compacted.read_file("etc/hostname").unwrap(),
            Some(b"edge-node-01".to_vec())
        );
        assert_eq!(compacted.read_file("var/cache/deleted").unwrap(), None);
        assert_ne!(compacted.filesystem_id(), fs.filesystem_id());

        let reopened = EdgeFs::open(compacted.into_device(), KEY).unwrap();
        assert_eq!(
            reopened.read_file("etc/hostname").unwrap(),
            Some(b"edge-node-01".to_vec())
        );
        assert_eq!(reopened.read_file("var/cache/deleted").unwrap(), None);
    }

    #[test]
    fn compact_into_preflights_target_capacity() {
        let source_device = InMemoryBlockDevice::new(512, 128);
        let target_device = InMemoryBlockDevice::new(512, 3);
        let untouched_target = target_device.clone();
        let mut fs = EdgeFs::format_with_id(source_device, KEY, [0x19; 16]).unwrap();
        fs.write_file("etc/hostname", b"edge-node-01", FileMeta::default())
            .unwrap();

        let error = match fs.compact_into(target_device, KEY, [0x1a; 16]) {
            Ok(_) => panic!("compact_into unexpectedly fit into a tiny target"),
            Err(error) => error,
        };

        assert!(matches!(error, EdgeFsError::OutOfSpace));
        assert!(matches!(
            EdgeFs::open(untouched_target, KEY),
            Err(EdgeFsError::NotFormatted)
        ));
    }
}
