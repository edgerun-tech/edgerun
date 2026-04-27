//! EdgeFS: encrypted, append-first block filesystem for Edgerun.
//!
//! This initial implementation deliberately keeps durable authority in an
//! encrypted append log. Public block bytes contain only filesystem geometry,
//! record sizes, nonces, and checkpoint cursor state. Paths, metadata, and file
//! contents are encrypted as AEAD payloads and indexes are rebuilt by scanning
//! the log.

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use edgerun_crypto::{Aead, AesGcmCipher, KeyInit, Nonce};
use edgerun_storage::BlockStorage;

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
const DEFAULT_MODE_FILE: u32 = 0o644;
const DEFAULT_MODE_DIR: u32 = 0o755;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeFsError {
    InvalidKey,
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
}

impl fmt::Display for EdgeFsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey => f.write_str("invalid EdgeFS encryption key"),
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
        }
    }
}

impl core::error::Error for EdgeFsError {}

impl From<edgerun_storage::StorageError> for EdgeFsError {
    fn from(value: edgerun_storage::StorageError) -> Self {
        Self::Device(value.to_string())
    }
}

pub type Result<T> = core::result::Result<T, EdgeFsError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMeta {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

impl FileMeta {
    pub const fn file(mode: u32, uid: u32, gid: u32) -> Self {
        Self { mode, uid, gid }
    }

    pub const fn dir(mode: u32, uid: u32, gid: u32) -> Self {
        Self { mode, uid, gid }
    }
}

impl Default for FileMeta {
    fn default() -> Self {
        Self {
            mode: DEFAULT_MODE_FILE,
            uid: 0,
            gid: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
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
            Self::Character => 4,
            Self::Block => 5,
            Self::Fifo => 6,
        }
    }

    fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::File),
            2 => Some(Self::Directory),
            3 => Some(Self::Symlink),
            4 => Some(Self::Character),
            5 => Some(Self::Block),
            6 => Some(Self::Fifo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub path: String,
    pub kind: EntryKind,
    pub meta: FileMeta,
    pub len: usize,
}

#[derive(Clone)]
struct IndexedEntry {
    kind: EntryKind,
    meta: FileMeta,
    data: Vec<u8>,
    link_target: Option<String>,
}

#[derive(Debug, Clone)]
struct Superblock {
    generation: u64,
    cursor: u64,
    sector_size: usize,
    sectors: u64,
    fs_id: [u8; 16],
}

/// A mandatory-encryption block filesystem over a sector-addressable device.
pub struct EdgeFs<S: BlockStorage> {
    device: S,
    key: [u8; KEY_LEN],
    fs_id: [u8; 16],
    sector_size: usize,
    sectors: u64,
    cursor: u64,
    generation: u64,
    entries: BTreeMap<String, IndexedEntry>,
}

impl<S: BlockStorage> EdgeFs<S> {
    pub fn format(mut device: S, key: [u8; KEY_LEN]) -> Result<Self> {
        validate_key(&key)?;
        let sector_size = device.sector_size();
        let sectors = device.sectors();
        validate_geometry(sector_size, sectors)?;

        let fs_id = derive_fs_id(&key, sector_size, sectors);
        let superblock = Superblock {
            generation: 1,
            cursor: DATA_START_SECTOR.saturating_mul(sector_size as u64),
            sector_size,
            sectors,
            fs_id,
        };
        write_superblocks(&mut device, &superblock)?;
        device.sync()?;

        Ok(Self {
            device,
            key,
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
        let superblock = read_best_superblock(&mut device, sector_size)?;
        if superblock.sector_size != sector_size || superblock.sectors != sectors {
            return Err(EdgeFsError::CorruptSuperblock(
                "stored geometry does not match device".into(),
            ));
        }

        let mut fs = Self {
            device,
            key,
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

    pub fn into_device(self) -> S {
        self.device
    }

    pub fn block_size(&self) -> usize {
        self.sector_size
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(normalize_path(path).as_str())
    }

    pub fn mkdir_all(&mut self, path: &str, meta: FileMeta) -> Result<()> {
        let path = normalize_path(path);
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
        })
    }

    pub fn remove_path(&mut self, path: &str) -> Result<()> {
        let path = normalize_non_empty_path(path)?;
        self.append_entry(RecordPayload::Delete { path })
    }

    pub fn remove_children(&mut self, path: &str) -> Result<()> {
        let path = normalize_path(path);
        self.append_entry(RecordPayload::DeleteChildren { path })
    }

    pub fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let path = normalize_path(path);
        Ok(self.entries.get(&path).and_then(|entry| {
            (entry.kind == EntryKind::File).then(|| entry.data.clone())
        }))
    }

    pub fn read_link(&self, path: &str) -> Result<Option<String>> {
        let path = normalize_path(path);
        Ok(self.entries.get(&path).and_then(|entry| {
            (entry.kind == EntryKind::Symlink)
                .then(|| entry.link_target.clone())
                .flatten()
        }))
    }

    pub fn list(&self) -> Vec<DirEntry> {
        self.entries
            .iter()
            .map(|(path, entry)| DirEntry {
                path: path.clone(),
                kind: entry.kind,
                meta: entry.meta,
                len: entry.data.len(),
            })
            .collect()
    }

    fn ensure_parent_dirs(&mut self, path: &str, meta: FileMeta) -> Result<()> {
        if let Some((parent, _)) = path.rsplit_once('/') {
            self.mkdir_all(parent, FileMeta::dir(DEFAULT_MODE_DIR, meta.uid, meta.gid))?;
        }
        Ok(())
    }

    fn append_entry(&mut self, payload: RecordPayload) -> Result<()> {
        let offset = self.cursor;
        let plaintext = encode_payload(&payload)?;
        let nonce = self.nonce_for_offset(offset);
        let aad = self.record_aad(offset);
        let ciphertext = encrypt_payload(&self.key, &nonce, &aad, &plaintext)?;
        let record_len = RECORD_HEADER_LEN + ciphertext.len();
        let padded_len = round_up(record_len, self.sector_size)?;

        if offset.saturating_add(padded_len as u64) > self.device_bytes() {
            return Err(EdgeFsError::OutOfSpace);
        }

        let mut record = vec![0u8; padded_len];
        encode_record_header(&mut record[..RECORD_HEADER_LEN], &nonce, plaintext.len(), ciphertext.len())?;
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
        write_superblocks(&mut self.device, &superblock)?;
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
            let plaintext = decrypt_payload(&self.key, &header.nonce, &aad, &ciphertext)?;
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
            } => {
                self.entries.insert(
                    path,
                    IndexedEntry {
                        kind,
                        meta,
                        data,
                        link_target,
                    },
                );
            }
            RecordPayload::Delete { path } => {
                self.entries.remove(&path);
                let prefix = format!("{path}/");
                self.entries.retain(|entry_path, _| !entry_path.starts_with(&prefix));
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
        let hash = edgerun_crypto::sha256(&input);
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
}

#[derive(Debug, Clone)]
enum RecordPayload {
    Put {
        path: String,
        kind: EntryKind,
        meta: FileMeta,
        data: Vec<u8>,
        link_target: Option<String>,
    },
    Delete {
        path: String,
    },
    DeleteChildren {
        path: String,
    },
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

fn derive_fs_id(key: &[u8; KEY_LEN], sector_size: usize, sectors: u64) -> [u8; 16] {
    let mut input = Vec::with_capacity(KEY_LEN + 8 + 8 + 9);
    input.extend_from_slice(key);
    input.extend_from_slice(&(sector_size as u64).to_le_bytes());
    input.extend_from_slice(&sectors.to_le_bytes());
    input.extend_from_slice(b"edgefs:id");
    let hash = edgerun_crypto::sha256(&input);
    let mut fs_id = [0u8; 16];
    fs_id.copy_from_slice(&hash[..16]);
    fs_id
}

fn write_superblocks<S: BlockStorage>(device: &mut S, sb: &Superblock) -> Result<()> {
    let mut sector = vec![0u8; sb.sector_size];
    sector[..8].copy_from_slice(SUPER_MAGIC);
    put_u16(&mut sector, 8, VERSION);
    put_u16(&mut sector, 10, 64);
    put_u64(&mut sector, 12, sb.generation);
    put_u64(&mut sector, 20, sb.cursor);
    put_u32(&mut sector, 28, sb.sector_size as u32);
    put_u64(&mut sector, 32, sb.sectors);
    sector[40..56].copy_from_slice(&sb.fs_id);
    let checksum = edgerun_crypto::sha256(&sector[..56]);
    sector[56..64].copy_from_slice(&checksum[..8]);
    device.write_sector(SUPERBLOCK_A, &sector)?;
    device.write_sector(SUPERBLOCK_B, &sector)?;
    Ok(())
}

fn read_best_superblock<S: BlockStorage>(device: &mut S, sector_size: usize) -> Result<Superblock> {
    let a = read_superblock_at(device, sector_size, SUPERBLOCK_A);
    let b = read_superblock_at(device, sector_size, SUPERBLOCK_B);
    match (a, b) {
        (Ok(left), Ok(right)) => Ok(if left.generation >= right.generation {
            left
        } else {
            right
        }),
        (Ok(sb), Err(_)) | (Err(_), Ok(sb)) => Ok(sb),
        (Err(_), Err(_)) => Err(EdgeFsError::NotFormatted),
    }
}

fn read_superblock_at<S: BlockStorage>(
    device: &mut S,
    sector_size: usize,
    sector_index: u64,
) -> Result<Superblock> {
    let mut sector = vec![0u8; sector_size];
    device.read_sector(sector_index, &mut sector)?;
    if &sector[..8] != SUPER_MAGIC {
        return Err(EdgeFsError::NotFormatted);
    }
    if get_u16(&sector, 8)? != VERSION {
        return Err(EdgeFsError::CorruptSuperblock(
            "unsupported version".into(),
        ));
    }
    let checksum = edgerun_crypto::sha256(&sector[..56]);
    if sector[56..64] != checksum[..8] {
        return Err(EdgeFsError::CorruptSuperblock(
            "checksum mismatch".into(),
        ));
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
    let mut out = Vec::new();
    match payload {
        RecordPayload::Put {
            path,
            kind,
            meta,
            data,
            link_target,
        } => {
            validate_normalized_path(path)?;
            let target = link_target.as_deref().unwrap_or("");
            out.push(1);
            out.push(kind.to_u8());
            put_u32_vec(&mut out, meta.mode);
            put_u32_vec(&mut out, meta.uid);
            put_u32_vec(&mut out, meta.gid);
            put_u32_vec(&mut out, path.len() as u32);
            put_u32_vec(&mut out, target.len() as u32);
            put_u64_vec(&mut out, data.len() as u64);
            out.extend_from_slice(path.as_bytes());
            out.extend_from_slice(target.as_bytes());
            out.extend_from_slice(data);
        }
        RecordPayload::Delete { path } => {
            validate_normalized_path(path)?;
            out.push(2);
            put_u32_vec(&mut out, path.len() as u32);
            out.extend_from_slice(path.as_bytes());
        }
        RecordPayload::DeleteChildren { path } => {
            out.push(3);
            put_u32_vec(&mut out, path.len() as u32);
            out.extend_from_slice(path.as_bytes());
        }
    }
    Ok(out)
}

fn decode_payload(input: &[u8]) -> Result<RecordPayload> {
    let Some((&tag, rest)) = input.split_first() else {
        return Err(EdgeFsError::CorruptRecord("empty payload".into()));
    };
    match tag {
        1 => decode_put_payload(rest),
        2 => {
            let (path_len, offset) = read_u32_at(rest, 0)?;
            let path = read_string(rest, offset, path_len as usize)?;
            Ok(RecordPayload::Delete { path })
        }
        3 => {
            let (path_len, offset) = read_u32_at(rest, 0)?;
            let path = read_string(rest, offset, path_len as usize)?;
            Ok(RecordPayload::DeleteChildren { path })
        }
        _ => Err(EdgeFsError::CorruptRecord("unknown payload tag".into())),
    }
}

fn decode_put_payload(input: &[u8]) -> Result<RecordPayload> {
    if input.len() < 30 {
        return Err(EdgeFsError::CorruptRecord("short put payload".into()));
    }
    let kind = EntryKind::from_u8(input[0])
        .ok_or_else(|| EdgeFsError::CorruptRecord("unknown entry kind".into()))?;
    let mut offset = 1usize;
    let (mode, next) = read_u32_at(input, offset)?;
    offset = next;
    let (uid, next) = read_u32_at(input, offset)?;
    offset = next;
    let (gid, next) = read_u32_at(input, offset)?;
    offset = next;
    let (path_len, next) = read_u32_at(input, offset)?;
    offset = next;
    let (target_len, next) = read_u32_at(input, offset)?;
    offset = next;
    let (data_len, next) = read_u64_at(input, offset)?;
    offset = next;

    let path = read_string(input, offset, path_len as usize)?;
    offset += path_len as usize;
    let target = read_string(input, offset, target_len as usize)?;
    offset += target_len as usize;
    let data_end = offset
        .checked_add(data_len as usize)
        .ok_or_else(|| EdgeFsError::CorruptRecord("data length overflow".into()))?;
    if data_end > input.len() {
        return Err(EdgeFsError::CorruptRecord("data extends past payload".into()));
    }

    Ok(RecordPayload::Put {
        path,
        kind,
        meta: FileMeta { mode, uid, gid },
        data: input[offset..data_end].to_vec(),
        link_target: (!target.is_empty()).then_some(target),
    })
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

fn normalize_path(path: &str) -> String {
    let mut out = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out.join("/")
}

fn normalize_non_empty_path(path: &str) -> Result<String> {
    let path = normalize_path(path);
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

fn put_u32_vec(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64_vec(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32_at(input: &[u8], offset: usize) -> Result<(u32, usize)> {
    Ok((get_u32(input, offset)?, offset + 4))
}

fn read_u64_at(input: &[u8], offset: usize) -> Result<(u64, usize)> {
    Ok((get_u64(input, offset)?, offset + 8))
}

fn read_string(input: &[u8], offset: usize, len: usize) -> Result<String> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| EdgeFsError::CorruptRecord("string length overflow".into()))?;
    let bytes = input
        .get(offset..end)
        .ok_or_else(|| EdgeFsError::CorruptRecord("string extends past payload".into()))?;
    core::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|_| EdgeFsError::CorruptRecord("string is not UTF-8".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_storage::InMemoryBlockDevice;

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
    fn writes_reads_and_reopens_encrypted_files() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format(device, KEY).unwrap();

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
        assert_eq!(fs.read_link("bin/sh").unwrap(), Some("/bin/busybox".into()));

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
        assert_eq!(reopened.read_link("bin/sh").unwrap(), Some("/bin/busybox".into()));
    }

    #[test]
    fn wrong_key_cannot_rebuild_index() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format(device, KEY).unwrap();
        fs.write_file("secret.txt", b"super-secret", FileMeta::default())
            .unwrap();
        let device = fs.into_device();

        let err = EdgeFs::open(device, [0x24; 32]).unwrap_err();
        assert!(matches!(err, EdgeFsError::Decryption));
    }

    #[test]
    fn delete_and_opaque_directory_replay() {
        let device = InMemoryBlockDevice::new(512, 128);
        let mut fs = EdgeFs::format(device, KEY).unwrap();
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
}
