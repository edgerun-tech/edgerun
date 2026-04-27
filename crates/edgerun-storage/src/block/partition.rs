//! no_std partition table detection and partition-backed block devices.

use crate::block::BlockStorage;
use crate::error::StorageError;
use crate::prelude::v1::*;

const MBR_SIGNATURE_OFFSET: usize = 510;
const MBR_PARTITION_OFFSET: usize = 446;
const MBR_PARTITION_LEN: usize = 16;
const MBR_PARTITION_COUNT: usize = 4;
const GPT_HEADER_LBA: u64 = 1;
const GPT_HEADER_SIGNATURE: &[u8; 8] = b"EFI PART";
const GPT_PARTITION_TYPE_UNUSED: [u8; 16] = [0u8; 16];

#[derive(Debug)]
pub enum PartitionError {
    SectorTooSmall(usize),
    Read(StorageError),
    InvalidRange {
        start_lba: u64,
        sectors: u64,
        disk_sectors: u64,
    },
}

impl core::fmt::Display for PartitionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SectorTooSmall(size) => write!(f, "sector size is too small: {size}"),
            Self::Read(error) => write!(f, "partition table read failed: {error}"),
            Self::InvalidRange {
                start_lba,
                sectors,
                disk_sectors,
            } => write!(
                f,
                "partition range {start_lba}+{sectors} exceeds disk sectors {disk_sectors}"
            ),
        }
    }
}

impl core::error::Error for PartitionError {}

impl From<StorageError> for PartitionError {
    fn from(error: StorageError) -> Self {
        Self::Read(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionTableKind {
    None,
    Mbr,
    Gpt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionTable {
    pub kind: PartitionTableKind,
    pub partitions: Vec<PartitionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionEntry {
    pub index: u32,
    pub start_lba: u64,
    pub sectors: u64,
    pub kind: PartitionKind,
    pub flags: u64,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionKind {
    Mbr {
        type_code: u8,
        bootable: bool,
    },
    Gpt {
        type_guid: [u8; 16],
        unique_guid: [u8; 16],
    },
}

pub struct PartitionBlockDevice<S: BlockStorage> {
    parent: S,
    start_lba: u64,
    sectors: u64,
}

impl<S: BlockStorage> PartitionBlockDevice<S> {
    pub fn new(parent: S, start_lba: u64, sectors: u64) -> Result<Self, PartitionError> {
        validate_partition_range(parent.sectors(), start_lba, sectors)?;
        Ok(Self {
            parent,
            start_lba,
            sectors,
        })
    }

    pub fn from_entry(parent: S, entry: &PartitionEntry) -> Result<Self, PartitionError> {
        Self::new(parent, entry.start_lba, entry.sectors)
    }

    pub fn into_parent(self) -> S {
        self.parent
    }

    pub fn start_lba(&self) -> u64 {
        self.start_lba
    }
}

impl<S: BlockStorage> BlockStorage for PartitionBlockDevice<S> {
    fn sector_size(&self) -> usize {
        self.parent.sector_size()
    }

    fn sectors(&self) -> u64 {
        self.sectors
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        let parent_sector = self.parent_sector(sector)?;
        self.parent.read_sector(parent_sector, buf)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), StorageError> {
        let parent_sector = self.parent_sector(sector)?;
        self.parent.write_sector(parent_sector, buf)
    }

    fn sync(&mut self) -> Result<(), StorageError> {
        self.parent.sync()
    }
}

impl<S: BlockStorage> PartitionBlockDevice<S> {
    fn parent_sector(&self, sector: u64) -> Result<u64, StorageError> {
        if sector >= self.sectors {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "partition sector index out of range",
            )));
        }
        self.start_lba
            .checked_add(sector)
            .ok_or_else(|| StorageError::Io(std::io::Error::other("partition sector overflow")))
    }
}

pub fn detect_partitions<S: BlockStorage>(
    device: &mut S,
) -> Result<PartitionTable, PartitionError> {
    let sector_size = device.sector_size();
    if sector_size < 512 {
        return Err(PartitionError::SectorTooSmall(sector_size));
    }

    let mut lba0 = vec![0u8; sector_size];
    device.read_sector(0, &mut lba0)?;
    if lba0[MBR_SIGNATURE_OFFSET] != 0x55 || lba0[MBR_SIGNATURE_OFFSET + 1] != 0xaa {
        return Ok(PartitionTable {
            kind: PartitionTableKind::None,
            partitions: Vec::new(),
        });
    }

    if has_protective_mbr(&lba0) {
        if let Some(table) = read_gpt(device)? {
            return Ok(table);
        }
    }

    Ok(PartitionTable {
        kind: PartitionTableKind::Mbr,
        partitions: read_mbr_partitions(&lba0, device.sectors())?,
    })
}

fn has_protective_mbr(lba0: &[u8]) -> bool {
    (0..MBR_PARTITION_COUNT)
        .any(|index| lba0[MBR_PARTITION_OFFSET + index * MBR_PARTITION_LEN + 4] == 0xee)
}

fn read_mbr_partitions(
    lba0: &[u8],
    disk_sectors: u64,
) -> Result<Vec<PartitionEntry>, PartitionError> {
    let mut partitions = Vec::new();
    for index in 0..MBR_PARTITION_COUNT {
        let offset = MBR_PARTITION_OFFSET + index * MBR_PARTITION_LEN;
        let type_code = lba0[offset + 4];
        let start_lba = read_u32(lba0, offset + 8) as u64;
        let sectors = read_u32(lba0, offset + 12) as u64;
        if type_code == 0 || sectors == 0 {
            continue;
        }
        validate_partition_range(disk_sectors, start_lba, sectors)?;
        partitions.push(PartitionEntry {
            index: (index + 1) as u32,
            start_lba,
            sectors,
            kind: PartitionKind::Mbr {
                type_code,
                bootable: lba0[offset] == 0x80,
            },
            flags: 0,
            name: None,
        });
    }
    Ok(partitions)
}

fn read_gpt<S: BlockStorage>(device: &mut S) -> Result<Option<PartitionTable>, PartitionError> {
    let sector_size = device.sector_size();
    let mut header = vec![0u8; sector_size];
    device.read_sector(GPT_HEADER_LBA, &mut header)?;
    if &header[..8] != GPT_HEADER_SIGNATURE {
        return Ok(None);
    }

    let entries_lba = read_u64(&header, 72);
    let entry_count = read_u32(&header, 80);
    let entry_size = read_u32(&header, 84) as usize;
    if entry_size < 128 {
        return Ok(Some(PartitionTable {
            kind: PartitionTableKind::Gpt,
            partitions: Vec::new(),
        }));
    }

    let mut partitions = Vec::new();
    let mut sector = vec![0u8; sector_size];
    for index in 0..entry_count {
        let byte_offset = index as u64 * entry_size as u64;
        let lba = entries_lba + byte_offset / sector_size as u64;
        let offset = (byte_offset % sector_size as u64) as usize;
        if offset + entry_size > sector_size {
            continue;
        }
        device.read_sector(lba, &mut sector)?;
        let entry = &sector[offset..offset + entry_size];
        let mut type_guid = [0u8; 16];
        type_guid.copy_from_slice(&entry[..16]);
        if type_guid == GPT_PARTITION_TYPE_UNUSED {
            continue;
        }
        let mut unique_guid = [0u8; 16];
        unique_guid.copy_from_slice(&entry[16..32]);
        let first_lba = read_u64(entry, 32);
        let last_lba = read_u64(entry, 40);
        let sectors = last_lba
            .checked_sub(first_lba)
            .and_then(|value| value.checked_add(1))
            .unwrap_or(0);
        if sectors == 0 {
            continue;
        }
        validate_partition_range(device.sectors(), first_lba, sectors)?;
        partitions.push(PartitionEntry {
            index: index + 1,
            start_lba: first_lba,
            sectors,
            kind: PartitionKind::Gpt {
                type_guid,
                unique_guid,
            },
            flags: read_u64(entry, 48),
            name: decode_gpt_name(&entry[56..core::cmp::min(entry.len(), 128)]),
        });
    }

    Ok(Some(PartitionTable {
        kind: PartitionTableKind::Gpt,
        partitions,
    }))
}

fn validate_partition_range(
    disk_sectors: u64,
    start_lba: u64,
    sectors: u64,
) -> Result<(), PartitionError> {
    if sectors == 0
        || start_lba
            .checked_add(sectors)
            .map(|end| end > disk_sectors)
            .unwrap_or(true)
    {
        Err(PartitionError::InvalidRange {
            start_lba,
            sectors,
            disk_sectors,
        })
    } else {
        Ok(())
    }
}

fn decode_gpt_name(input: &[u8]) -> Option<String> {
    let mut out = String::new();
    for chunk in input.chunks_exact(2) {
        let value = u16::from_le_bytes([chunk[0], chunk[1]]);
        if value == 0 {
            break;
        }
        if let Some(ch) = char::from_u32(value as u32) {
            out.push(ch);
        }
    }
    (!out.is_empty()).then_some(out)
}

fn read_u32(input: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

fn read_u64(input: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
        input[offset + 4],
        input[offset + 5],
        input[offset + 6],
        input[offset + 7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::InMemoryBlockDevice;

    #[test]
    fn detects_mbr_partitions() {
        let mut device = InMemoryBlockDevice::new(512, 128);
        let mut mbr = vec![0u8; 512];
        mbr[510] = 0x55;
        mbr[511] = 0xaa;
        mbr[446] = 0x80;
        mbr[450] = 0x83;
        mbr[454..458].copy_from_slice(&8u32.to_le_bytes());
        mbr[458..462].copy_from_slice(&32u32.to_le_bytes());
        device.write_sector(0, &mbr).unwrap();

        let table = detect_partitions(&mut device).unwrap();
        assert_eq!(table.kind, PartitionTableKind::Mbr);
        assert_eq!(table.partitions.len(), 1);
        assert_eq!(table.partitions[0].start_lba, 8);
        assert_eq!(table.partitions[0].sectors, 32);
        assert_eq!(
            table.partitions[0].kind,
            PartitionKind::Mbr {
                type_code: 0x83,
                bootable: true
            }
        );
    }

    #[test]
    fn detects_gpt_partitions() {
        let mut device = InMemoryBlockDevice::new(512, 256);
        let mut mbr = vec![0u8; 512];
        mbr[510] = 0x55;
        mbr[511] = 0xaa;
        mbr[450] = 0xee;
        mbr[454..458].copy_from_slice(&1u32.to_le_bytes());
        mbr[458..462].copy_from_slice(&255u32.to_le_bytes());
        device.write_sector(0, &mbr).unwrap();

        let mut header = vec![0u8; 512];
        header[..8].copy_from_slice(GPT_HEADER_SIGNATURE);
        header[72..80].copy_from_slice(&2u64.to_le_bytes());
        header[80..84].copy_from_slice(&128u32.to_le_bytes());
        header[84..88].copy_from_slice(&128u32.to_le_bytes());
        device.write_sector(1, &header).unwrap();

        let mut entries = vec![0u8; 512];
        entries[0] = 0xaf;
        entries[16] = 0x11;
        entries[32..40].copy_from_slice(&40u64.to_le_bytes());
        entries[40..48].copy_from_slice(&99u64.to_le_bytes());
        for (index, value) in "edgefs".encode_utf16().enumerate() {
            let offset = 56 + index * 2;
            entries[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
        device.write_sector(2, &entries).unwrap();

        let table = detect_partitions(&mut device).unwrap();
        assert_eq!(table.kind, PartitionTableKind::Gpt);
        assert_eq!(table.partitions.len(), 1);
        assert_eq!(table.partitions[0].start_lba, 40);
        assert_eq!(table.partitions[0].sectors, 60);
        assert_eq!(table.partitions[0].name.as_deref(), Some("edgefs"));
    }

    #[test]
    fn partition_device_maps_relative_sectors() {
        let mut parent = InMemoryBlockDevice::new(512, 16);
        let data = vec![0x5au8; 512];
        parent.write_sector(5, &data).unwrap();

        let mut partition = PartitionBlockDevice::new(parent, 5, 4).unwrap();
        let mut out = vec![0u8; 512];
        partition.read_sector(0, &mut out).unwrap();
        assert_eq!(out, data);
    }
}
