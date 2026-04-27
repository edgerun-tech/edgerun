//! Minimal no_std read-only FAT root directory support.

use crate::block::{probe_filesystem, BlockStorage, FatInfo, FileSystemDetails, FileSystemKind};
use crate::error::StorageError;
use crate::prelude::v1::*;

#[derive(Debug)]
pub enum FatError {
    Storage(StorageError),
    NotFat,
    UnsupportedFat(FileSystemKind),
    InvalidGeometry,
    InvalidPath(String),
    NotFound(String),
    DirectoryOnly,
    NotDirectory(String),
}

impl core::fmt::Display for FatError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(error) => write!(f, "FAT storage error: {error}"),
            Self::NotFat => f.write_str("device is not a supported FAT filesystem"),
            Self::UnsupportedFat(kind) => write!(f, "unsupported FAT variant: {kind:?}"),
            Self::InvalidGeometry => f.write_str("invalid FAT geometry"),
            Self::InvalidPath(path) => write!(f, "invalid FAT path: {path}"),
            Self::NotFound(path) => write!(f, "FAT file not found: {path}"),
            Self::DirectoryOnly => f.write_str("FAT entry is a directory"),
            Self::NotDirectory(path) => write!(f, "FAT entry is not a directory: {path}"),
        }
    }
}

impl core::error::Error for FatError {}

impl From<StorageError> for FatError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FatDirectoryEntry {
    pub name: String,
    pub attr: u8,
    pub first_cluster: u32,
    pub size: u32,
}

impl FatDirectoryEntry {
    pub fn is_directory(&self) -> bool {
        self.attr & 0x10 != 0
    }
}

pub struct FatReadOnly<S: BlockStorage> {
    device: S,
    kind: FileSystemKind,
    info: FatInfo,
    first_data_sector: u64,
    root_dir_sector: u64,
    root_dir_sectors: u64,
}

impl<S: BlockStorage> FatReadOnly<S> {
    pub fn open(mut device: S) -> Result<Self, FatError> {
        let probe = probe_filesystem(&mut device).map_err(|_| FatError::NotFat)?;
        let kind = probe.kind;
        if !matches!(
            kind,
            FileSystemKind::Fat12 | FileSystemKind::Fat16 | FileSystemKind::Fat32
        ) {
            return Err(FatError::UnsupportedFat(kind));
        }
        let Some(FileSystemDetails::Fat(info)) = probe.details else {
            return Err(FatError::NotFat);
        };
        validate_info(&info)?;

        let root_dir_sectors = ((info.root_entry_count as u32 * 32) + info.bytes_per_sector as u32
            - 1)
            / info.bytes_per_sector as u32;
        let root_dir_sector =
            info.reserved_sectors as u64 + info.fat_count as u64 * info.fat_size_sectors as u64;
        let first_data_sector = root_dir_sector + root_dir_sectors as u64;

        Ok(Self {
            device,
            kind,
            info,
            first_data_sector,
            root_dir_sector,
            root_dir_sectors: root_dir_sectors as u64,
        })
    }

    pub fn into_device(self) -> S {
        self.device
    }

    pub fn root_entries(&mut self) -> Result<Vec<FatDirectoryEntry>, FatError> {
        match self.kind {
            FileSystemKind::Fat12 | FileSystemKind::Fat16 => {
                self.read_directory_sectors(self.root_dir_sector, self.root_dir_sectors)
            }
            FileSystemKind::Fat32 => {
                let root = self.info.root_cluster.ok_or(FatError::InvalidGeometry)?;
                self.read_directory_cluster_chain(root)
            }
            kind => Err(FatError::UnsupportedFat(kind)),
        }
    }

    pub fn read_root_file_8_3(&mut self, name: &str) -> Result<Vec<u8>, FatError> {
        self.read_file(name)
    }

    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>, FatError> {
        let entry = self.resolve_path(path)?;
        if entry.is_directory() {
            return Err(FatError::DirectoryOnly);
        }
        self.read_file_clusters(entry.first_cluster, entry.size as usize)
    }

    pub fn list_dir(&mut self, path: &str) -> Result<Vec<FatDirectoryEntry>, FatError> {
        let trimmed = trim_path(path);
        if trimmed.is_empty() {
            return self.root_entries();
        }
        let entry = self.resolve_path(path)?;
        if !entry.is_directory() {
            return Err(FatError::NotDirectory(path.into()));
        }
        self.read_directory_entry(&entry)
    }

    pub fn read_file_8_3(&mut self, path: &str) -> Result<Vec<u8>, FatError> {
        self.read_file(path)
    }

    pub fn list_dir_8_3(&mut self, path: &str) -> Result<Vec<FatDirectoryEntry>, FatError> {
        self.list_dir(path)
    }

    fn resolve_path(&mut self, path: &str) -> Result<FatDirectoryEntry, FatError> {
        let trimmed = trim_path(path);
        if trimmed.is_empty() {
            return Err(FatError::InvalidPath(path.into()));
        }

        let mut entries = self.root_entries()?;
        let mut current = None;
        for component in trimmed.split('/') {
            if component.is_empty() || component == "." || component == ".." {
                return Err(FatError::InvalidPath(path.into()));
            }
            let wanted = normalize_8_3(component);
            let entry = entries
                .into_iter()
                .find(|entry| normalize_8_3(entry.name.as_str()) == wanted)
                .ok_or_else(|| FatError::NotFound(path.into()))?;
            current = Some(entry.clone());
            entries = if entry.is_directory() {
                self.read_directory_entry(&entry)?
            } else {
                Vec::new()
            };
        }
        current.ok_or_else(|| FatError::InvalidPath(path.into()))
    }

    fn read_directory_entry(
        &mut self,
        entry: &FatDirectoryEntry,
    ) -> Result<Vec<FatDirectoryEntry>, FatError> {
        if !entry.is_directory() {
            return Err(FatError::NotDirectory(entry.name.clone()));
        }
        self.read_directory_cluster_chain(entry.first_cluster)
    }

    fn read_directory_cluster_chain(
        &mut self,
        first_cluster: u32,
    ) -> Result<Vec<FatDirectoryEntry>, FatError> {
        let mut entries = Vec::new();
        let mut cluster = first_cluster;
        while !self.is_eoc(cluster) {
            let mut data = vec![0u8; self.cluster_size()];
            self.read_cluster(cluster, &mut data)?;
            append_entries(&mut entries, &data);
            let next = self.next_cluster(cluster)?;
            if next == cluster || next < 2 {
                break;
            }
            cluster = next;
        }
        Ok(entries)
    }

    fn read_directory_sectors(
        &mut self,
        start_sector: u64,
        sectors: u64,
    ) -> Result<Vec<FatDirectoryEntry>, FatError> {
        let mut data = vec![0u8; sectors as usize * self.info.bytes_per_sector as usize];
        self.read_sectors(start_sector, &mut data)?;
        let mut entries = Vec::new();
        append_entries(&mut entries, &data);
        Ok(entries)
    }

    fn read_file_clusters(&mut self, first_cluster: u32, size: usize) -> Result<Vec<u8>, FatError> {
        if size == 0 {
            return Ok(Vec::new());
        }
        let mut out = Vec::with_capacity(size);
        let mut cluster = first_cluster;
        while out.len() < size && !self.is_eoc(cluster) && cluster >= 2 {
            let mut data = vec![0u8; self.cluster_size()];
            self.read_cluster(cluster, &mut data)?;
            let copy_len = core::cmp::min(size - out.len(), data.len());
            out.extend_from_slice(&data[..copy_len]);
            let next = self.next_cluster(cluster)?;
            if next == cluster {
                break;
            }
            cluster = next;
        }
        Ok(out)
    }

    fn read_cluster(&mut self, cluster: u32, out: &mut [u8]) -> Result<(), FatError> {
        let sector = self
            .first_data_sector
            .checked_add((cluster as u64).saturating_sub(2) * self.info.sectors_per_cluster as u64)
            .ok_or(FatError::InvalidGeometry)?;
        self.read_sectors(sector, out)
    }

    fn read_sectors(&mut self, start_sector: u64, out: &mut [u8]) -> Result<(), FatError> {
        let sector_size = self.info.bytes_per_sector as usize;
        if out.len() % sector_size != 0 {
            return Err(FatError::InvalidGeometry);
        }
        for (index, chunk) in out.chunks_mut(sector_size).enumerate() {
            self.device
                .read_sector(start_sector + index as u64, chunk)?;
        }
        Ok(())
    }

    fn next_cluster(&mut self, cluster: u32) -> Result<u32, FatError> {
        let bps = self.info.bytes_per_sector as u64;
        let fat_offset = match self.kind {
            FileSystemKind::Fat16 => cluster as u64 * 2,
            FileSystemKind::Fat32 => cluster as u64 * 4,
            FileSystemKind::Fat12 => return Ok(0x0fff),
            kind => return Err(FatError::UnsupportedFat(kind)),
        };
        let sector = self.info.reserved_sectors as u64 + fat_offset / bps;
        let offset = (fat_offset % bps) as usize;
        let mut buf = vec![0u8; self.info.bytes_per_sector as usize];
        self.device.read_sector(sector, &mut buf)?;
        Ok(match self.kind {
            FileSystemKind::Fat16 => read_u16(&buf, offset) as u32,
            FileSystemKind::Fat32 => read_u32(&buf, offset) & 0x0fff_ffff,
            _ => 0,
        })
    }

    fn is_eoc(&self, cluster: u32) -> bool {
        match self.kind {
            FileSystemKind::Fat12 => cluster >= 0x0ff8,
            FileSystemKind::Fat16 => cluster >= 0xfff8,
            FileSystemKind::Fat32 => cluster >= 0x0fff_fff8,
            _ => true,
        }
    }

    fn cluster_size(&self) -> usize {
        self.info.bytes_per_sector as usize * self.info.sectors_per_cluster as usize
    }
}

fn validate_info(info: &FatInfo) -> Result<(), FatError> {
    if info.bytes_per_sector == 0
        || info.sectors_per_cluster == 0
        || info.fat_count == 0
        || info.fat_size_sectors == 0
    {
        Err(FatError::InvalidGeometry)
    } else {
        Ok(())
    }
}

fn append_entries(out: &mut Vec<FatDirectoryEntry>, data: &[u8]) {
    let mut long_name = Vec::<u16>::new();
    for entry in data.chunks_exact(32) {
        if entry[0] == 0x00 {
            break;
        }
        if entry[0] == 0xe5 {
            long_name.clear();
            continue;
        }
        if entry[11] == 0x0f {
            let part = long_name_part(entry);
            long_name.splice(0..0, part);
            continue;
        }
        let name = if long_name.is_empty() {
            short_name(entry)
        } else {
            let decoded = decode_utf16_name(&long_name).or_else(|| short_name(entry));
            long_name.clear();
            decoded
        };
        if let Some(name) = name {
            out.push(FatDirectoryEntry {
                name,
                attr: entry[11],
                first_cluster: ((read_u16(entry, 20) as u32) << 16) | read_u16(entry, 26) as u32,
                size: read_u32(entry, 28),
            });
        }
    }
}

fn long_name_part(entry: &[u8]) -> Vec<u16> {
    let mut out = Vec::with_capacity(13);
    for offset in [1usize, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30] {
        let value = read_u16(entry, offset);
        if value == 0x0000 || value == 0xffff {
            break;
        }
        out.push(value);
    }
    out
}

fn decode_utf16_name(input: &[u16]) -> Option<String> {
    let mut out = String::new();
    for item in char::decode_utf16(input.iter().copied()) {
        out.push(item.ok()?);
    }
    (!out.is_empty()).then_some(out)
}

fn short_name(entry: &[u8]) -> Option<String> {
    let base = ascii_trim(&entry[..8])?;
    let ext = ascii_trim(&entry[8..11]);
    let mut out = base;
    if let Some(ext) = ext {
        out.push('.');
        out.push_str(ext.as_str());
    }
    Some(out)
}

fn ascii_trim(input: &[u8]) -> Option<String> {
    let end = input
        .iter()
        .rposition(|byte| *byte != b' ')
        .map(|index| index + 1)
        .unwrap_or(0);
    if end == 0 || input[..end].iter().any(|byte| !byte.is_ascii()) {
        return None;
    }
    core::str::from_utf8(&input[..end])
        .ok()
        .map(|value| value.into())
}

fn normalize_8_3(name: &str) -> String {
    name.bytes()
        .filter(|byte| *byte != b' ')
        .map(|byte| byte.to_ascii_uppercase() as char)
        .collect()
}

fn trim_path(path: &str) -> &str {
    path.trim_matches('/')
}

fn read_u16(input: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([input[offset], input[offset + 1]])
}

fn read_u32(input: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::InMemoryBlockDevice;

    #[test]
    fn reads_fat16_root_file() {
        let mut device = InMemoryBlockDevice::new(512, 32);
        let mut boot = vec![0u8; 512];
        boot[11..13].copy_from_slice(&512u16.to_le_bytes());
        boot[13] = 1;
        boot[14..16].copy_from_slice(&1u16.to_le_bytes());
        boot[16] = 1;
        boot[17..19].copy_from_slice(&16u16.to_le_bytes());
        boot[19..21].copy_from_slice(&32u16.to_le_bytes());
        boot[22..24].copy_from_slice(&1u16.to_le_bytes());
        boot[54..62].copy_from_slice(b"FAT16   ");
        device.write_sector(0, &boot).unwrap();

        let mut fat = vec![0u8; 512];
        fat[4..6].copy_from_slice(&0xffffu16.to_le_bytes());
        device.write_sector(1, &fat).unwrap();

        let mut root = vec![0u8; 512];
        root[0..11].copy_from_slice(b"HELLO   TXT");
        root[11] = 0x20;
        root[26..28].copy_from_slice(&2u16.to_le_bytes());
        root[28..32].copy_from_slice(&5u32.to_le_bytes());
        device.write_sector(2, &root).unwrap();

        let mut data = vec![0u8; 512];
        data[..5].copy_from_slice(b"hello");
        device.write_sector(3, &data).unwrap();

        let mut fat = FatReadOnly::open(device).unwrap();
        assert_eq!(fat.root_entries().unwrap()[0].name, "HELLO.TXT");
        assert_eq!(fat.read_root_file_8_3("hello.txt").unwrap(), b"hello");
    }

    #[test]
    fn reads_nested_fat16_short_name_file() {
        let mut device = InMemoryBlockDevice::new(512, 32);
        let mut boot = vec![0u8; 512];
        boot[11..13].copy_from_slice(&512u16.to_le_bytes());
        boot[13] = 1;
        boot[14..16].copy_from_slice(&1u16.to_le_bytes());
        boot[16] = 1;
        boot[17..19].copy_from_slice(&16u16.to_le_bytes());
        boot[19..21].copy_from_slice(&32u16.to_le_bytes());
        boot[22..24].copy_from_slice(&1u16.to_le_bytes());
        boot[54..62].copy_from_slice(b"FAT16   ");
        device.write_sector(0, &boot).unwrap();

        let mut fat = vec![0u8; 512];
        fat[6..8].copy_from_slice(&0xffffu16.to_le_bytes());
        fat[8..10].copy_from_slice(&0xffffu16.to_le_bytes());
        device.write_sector(1, &fat).unwrap();

        let mut root = vec![0u8; 512];
        root[0..11].copy_from_slice(b"CONFIG     ");
        root[11] = 0x10;
        root[26..28].copy_from_slice(&3u16.to_le_bytes());
        device.write_sector(2, &root).unwrap();

        let mut dir = vec![0u8; 512];
        dir[0..11].copy_from_slice(b"IMAGE   TXT");
        dir[11] = 0x20;
        dir[26..28].copy_from_slice(&4u16.to_le_bytes());
        dir[28..32].copy_from_slice(&6u32.to_le_bytes());
        device.write_sector(4, &dir).unwrap();

        let mut data = vec![0u8; 512];
        data[..6].copy_from_slice(b"alpine");
        device.write_sector(5, &data).unwrap();

        let mut fat = FatReadOnly::open(device).unwrap();
        assert_eq!(fat.list_dir_8_3("/CONFIG").unwrap()[0].name, "IMAGE.TXT");
        assert_eq!(fat.read_file_8_3("/config/image.txt").unwrap(), b"alpine");
    }

    #[test]
    fn reads_fat16_long_filename_entry() {
        let mut device = InMemoryBlockDevice::new(512, 32);
        let mut boot = vec![0u8; 512];
        boot[11..13].copy_from_slice(&512u16.to_le_bytes());
        boot[13] = 1;
        boot[14..16].copy_from_slice(&1u16.to_le_bytes());
        boot[16] = 1;
        boot[17..19].copy_from_slice(&16u16.to_le_bytes());
        boot[19..21].copy_from_slice(&32u16.to_le_bytes());
        boot[22..24].copy_from_slice(&1u16.to_le_bytes());
        boot[54..62].copy_from_slice(b"FAT16   ");
        device.write_sector(0, &boot).unwrap();

        let mut fat = vec![0u8; 512];
        fat[4..6].copy_from_slice(&0xffffu16.to_le_bytes());
        device.write_sector(1, &fat).unwrap();

        let mut root = vec![0u8; 512];
        write_lfn_entry(&mut root[0..32], "image-ref.txt");
        root[32..43].copy_from_slice(b"IMAGER~1TXT");
        root[43] = 0x20;
        root[58..60].copy_from_slice(&2u16.to_le_bytes());
        root[60..64].copy_from_slice(&9u32.to_le_bytes());
        device.write_sector(2, &root).unwrap();

        let mut data = vec![0u8; 512];
        data[..9].copy_from_slice(b"edge:test");
        device.write_sector(3, &data).unwrap();

        let mut fat = FatReadOnly::open(device).unwrap();
        assert_eq!(fat.root_entries().unwrap()[0].name, "image-ref.txt");
        assert_eq!(fat.read_file_8_3("/image-ref.txt").unwrap(), b"edge:test");
    }

    fn write_lfn_entry(out: &mut [u8], name: &str) {
        out.fill(0xff);
        out[0] = 0x41;
        out[11] = 0x0f;
        out[12] = 0;
        out[13] = 0;
        out[26] = 0;
        out[27] = 0;
        for (index, value) in name
            .encode_utf16()
            .chain(core::iter::once(0))
            .take(13)
            .enumerate()
        {
            let offset = [1usize, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30][index];
            out[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
    }
}
