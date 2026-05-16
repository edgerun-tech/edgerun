//! no_std block filesystem signature probing.

use crate::block::BlockStorage;
use crate::error::StorageError;
use crate::prelude::v1::*;
use edgerun_encoding::byteorder::{read_u16_le as read_u16, read_u32_le as read_u32};

const EDGEFS_MAGIC: &[u8; 8] = b"EDGEFS01";
const ISO9660_MAGIC: &[u8; 5] = b"CD001";

#[derive(Debug)]
pub enum FileSystemProbeError {
    SectorTooSmall(usize),
    Read(StorageError),
}

impl core::fmt::Display for FileSystemProbeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SectorTooSmall(size) => write!(f, "sector size is too small: {size}"),
            Self::Read(error) => write!(f, "filesystem probe read failed: {error}"),
        }
    }
}

impl core::error::Error for FileSystemProbeError {}

impl From<StorageError> for FileSystemProbeError {
    fn from(error: StorageError) -> Self {
        Self::Read(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemKind {
    Unknown,
    EdgeFs,
    Fat12,
    Fat16,
    Fat32,
    ExFat,
    Ext,
    Iso9660,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSystemProbe {
    pub kind: FileSystemKind,
    pub label: Option<String>,
    pub details: Option<FileSystemDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSystemDetails {
    Fat(FatInfo),
    ExFat(ExFatInfo),
    Ext(ExtInfo),
    Iso9660(Iso9660Info),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FatInfo {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub root_entry_count: u16,
    pub total_sectors: u32,
    pub fat_size_sectors: u32,
    pub root_cluster: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExFatInfo {
    pub bytes_per_sector: u32,
    pub sectors_per_cluster: u32,
    pub cluster_count: u32,
    pub root_cluster: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtInfo {
    pub inodes_count: u32,
    pub blocks_count: u64,
    pub block_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iso9660Info {
    pub volume_id: Option<String>,
}

pub fn probe_filesystem<S: BlockStorage>(
    device: &mut S,
) -> Result<FileSystemProbe, FileSystemProbeError> {
    let sector_size = device.sector_size();
    if sector_size < 512 {
        return Err(FileSystemProbeError::SectorTooSmall(sector_size));
    }

    let mut sector = vec![0u8; sector_size];
    device.read_sector(0, &mut sector)?;
    if sector.starts_with(EDGEFS_MAGIC) {
        return Ok(FileSystemProbe {
            kind: FileSystemKind::EdgeFs,
            label: None,
            details: None,
        });
    }
    if sector.get(3..11) == Some(b"EXFAT   ".as_slice()) {
        return Ok(FileSystemProbe {
            kind: FileSystemKind::ExFat,
            label: None,
            details: Some(FileSystemDetails::ExFat(exfat_info(&sector))),
        });
    }
    if sector.get(82..90) == Some(b"FAT32   ".as_slice()) {
        return Ok(FileSystemProbe {
            kind: FileSystemKind::Fat32,
            label: utf8_label(sector.get(71..82).unwrap_or(&[])),
            details: Some(FileSystemDetails::Fat(fat_info(&sector, true))),
        });
    }
    if sector.get(54..62) == Some(b"FAT12   ".as_slice()) {
        return Ok(FileSystemProbe {
            kind: FileSystemKind::Fat12,
            label: utf8_label(sector.get(43..54).unwrap_or(&[])),
            details: Some(FileSystemDetails::Fat(fat_info(&sector, false))),
        });
    }
    if sector.get(54..62) == Some(b"FAT16   ".as_slice()) {
        return Ok(FileSystemProbe {
            kind: FileSystemKind::Fat16,
            label: utf8_label(sector.get(43..54).unwrap_or(&[])),
            details: Some(FileSystemDetails::Fat(fat_info(&sector, false))),
        });
    }

    if let Some(ext) = probe_ext(device)? {
        return Ok(ext);
    }
    if let Some(iso) = probe_iso9660(device)? {
        return Ok(iso);
    }

    Ok(FileSystemProbe {
        kind: FileSystemKind::Unknown,
        label: None,
        details: None,
    })
}

fn probe_ext<S: BlockStorage>(
    device: &mut S,
) -> Result<Option<FileSystemProbe>, FileSystemProbeError> {
    let sector_size = device.sector_size();
    let byte_offset = 1024 + 56;
    let sector_index = byte_offset / sector_size;
    if sector_index as u64 >= device.sectors() {
        return Ok(None);
    }
    let mut sector = vec![0u8; sector_size];
    device.read_sector(sector_index as u64, &mut sector)?;
    let offset = byte_offset % sector_size;
    if sector.get(offset..offset + 2) == Some(&[0x53, 0xef]) {
        let label_offset = 1024 + 120 - sector_index * sector_size;
        let inodes_count = read_u32(&sector, 1024 - sector_index * sector_size);
        let blocks_low = read_u32(&sector, 1024 + 4 - sector_index * sector_size) as u64;
        let log_block_size = read_u32(&sector, 1024 + 24 - sector_index * sector_size);
        let block_size = 1024u32.checked_shl(log_block_size).unwrap_or(0);
        return Ok(Some(FileSystemProbe {
            kind: FileSystemKind::Ext,
            label: utf8_label(sector.get(label_offset..label_offset + 16).unwrap_or(&[])),
            details: Some(FileSystemDetails::Ext(ExtInfo {
                inodes_count,
                blocks_count: blocks_low,
                block_size,
            })),
        }));
    }
    Ok(None)
}

fn probe_iso9660<S: BlockStorage>(
    device: &mut S,
) -> Result<Option<FileSystemProbe>, FileSystemProbeError> {
    let sector_size = device.sector_size();
    if sector_size == 0 {
        return Ok(None);
    }
    let byte_offset = 0x8000 + 1;
    let sector_index = byte_offset / sector_size;
    if sector_index as u64 >= device.sectors() {
        return Ok(None);
    }
    let mut sector = vec![0u8; sector_size];
    device.read_sector(sector_index as u64, &mut sector)?;
    let offset = byte_offset % sector_size;
    if sector.get(offset..offset + ISO9660_MAGIC.len()) == Some(ISO9660_MAGIC.as_slice()) {
        let descriptor_start = 0x8000 - sector_index * sector_size;
        let volume_id = utf8_label(
            sector
                .get(descriptor_start + 40..descriptor_start + 72)
                .unwrap_or(&[]),
        );
        return Ok(Some(FileSystemProbe {
            kind: FileSystemKind::Iso9660,
            label: volume_id.clone(),
            details: Some(FileSystemDetails::Iso9660(Iso9660Info { volume_id })),
        }));
    }
    Ok(None)
}

fn fat_info(sector: &[u8], fat32: bool) -> FatInfo {
    let total16 = read_u16(sector, 19) as u32;
    let fat16 = read_u16(sector, 22) as u32;
    FatInfo {
        bytes_per_sector: read_u16(sector, 11),
        sectors_per_cluster: sector[13],
        reserved_sectors: read_u16(sector, 14),
        fat_count: sector[16],
        root_entry_count: read_u16(sector, 17),
        total_sectors: if total16 != 0 {
            total16
        } else {
            read_u32(sector, 32)
        },
        fat_size_sectors: if fat16 != 0 {
            fat16
        } else {
            read_u32(sector, 36)
        },
        root_cluster: fat32.then(|| read_u32(sector, 44)),
    }
}

fn exfat_info(sector: &[u8]) -> ExFatInfo {
    let bytes_per_sector_shift = sector[108];
    let sectors_per_cluster_shift = sector[109];
    ExFatInfo {
        bytes_per_sector: 1u32.checked_shl(bytes_per_sector_shift as u32).unwrap_or(0),
        sectors_per_cluster: 1u32
            .checked_shl(sectors_per_cluster_shift as u32)
            .unwrap_or(0),
        cluster_count: read_u32(sector, 92),
        root_cluster: read_u32(sector, 96),
    }
}

fn utf8_label(input: &[u8]) -> Option<String> {
    let end = input
        .iter()
        .rposition(|byte| *byte != 0 && *byte != b' ')
        .map(|index| index + 1)
        .unwrap_or(0);
    if end == 0 {
        return None;
    }
    core::str::from_utf8(&input[..end])
        .ok()
        .map(|value| value.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::InMemoryBlockDevice;

    #[test]
    fn probes_edgefs_signature() {
        let mut device = InMemoryBlockDevice::new(512, 8);
        let mut sector = vec![0u8; 512];
        sector[..8].copy_from_slice(EDGEFS_MAGIC);
        device.write_sector(0, &sector).unwrap();

        let probe = probe_filesystem(&mut device).unwrap();
        assert_eq!(probe.kind, FileSystemKind::EdgeFs);
    }

    #[test]
    fn probes_fat32_signature_and_label() {
        let mut device = InMemoryBlockDevice::new(512, 8);
        let mut sector = vec![0u8; 512];
        sector[11..13].copy_from_slice(&512u16.to_le_bytes());
        sector[13] = 8;
        sector[14..16].copy_from_slice(&32u16.to_le_bytes());
        sector[16] = 2;
        sector[32..36].copy_from_slice(&1024u32.to_le_bytes());
        sector[36..40].copy_from_slice(&64u32.to_le_bytes());
        sector[44..48].copy_from_slice(&2u32.to_le_bytes());
        sector[82..90].copy_from_slice(b"FAT32   ");
        sector[71..82].copy_from_slice(b"EDGEBOOT   ");
        device.write_sector(0, &sector).unwrap();

        let probe = probe_filesystem(&mut device).unwrap();
        assert_eq!(probe.kind, FileSystemKind::Fat32);
        assert_eq!(probe.label.as_deref(), Some("EDGEBOOT"));
        assert_eq!(
            probe.details,
            Some(FileSystemDetails::Fat(FatInfo {
                bytes_per_sector: 512,
                sectors_per_cluster: 8,
                reserved_sectors: 32,
                fat_count: 2,
                root_entry_count: 0,
                total_sectors: 1024,
                fat_size_sectors: 64,
                root_cluster: Some(2),
            }))
        );
    }

    #[test]
    fn probes_ext_signature() {
        let mut device = InMemoryBlockDevice::new(512, 8);
        let mut sector = vec![0u8; 512];
        sector[0..4].copy_from_slice(&128u32.to_le_bytes());
        sector[4..8].copy_from_slice(&4096u32.to_le_bytes());
        sector[24..28].copy_from_slice(&2u32.to_le_bytes());
        sector[56..58].copy_from_slice(&[0x53, 0xef]);
        sector[120..124].copy_from_slice(b"root");
        device.write_sector(2, &sector).unwrap();

        let probe = probe_filesystem(&mut device).unwrap();
        assert_eq!(probe.kind, FileSystemKind::Ext);
        assert_eq!(probe.label.as_deref(), Some("root"));
        assert_eq!(
            probe.details,
            Some(FileSystemDetails::Ext(ExtInfo {
                inodes_count: 128,
                blocks_count: 4096,
                block_size: 4096,
            }))
        );
    }

    #[test]
    fn probes_iso9660_volume_id() {
        let mut device = InMemoryBlockDevice::new(512, 80);
        let mut sector = vec![0u8; 512];
        sector[1..6].copy_from_slice(ISO9660_MAGIC);
        sector[40..48].copy_from_slice(b"EDGEISO ");
        device.write_sector(64, &sector).unwrap();

        let probe = probe_filesystem(&mut device).unwrap();
        assert_eq!(probe.kind, FileSystemKind::Iso9660);
        assert_eq!(probe.label.as_deref(), Some("EDGEISO"));
    }

    #[test]
    fn probes_exfat_geometry() {
        let mut device = InMemoryBlockDevice::new(512, 8);
        let mut sector = vec![0u8; 512];
        sector[3..11].copy_from_slice(b"EXFAT   ");
        sector[92..96].copy_from_slice(&256u32.to_le_bytes());
        sector[96..100].copy_from_slice(&5u32.to_le_bytes());
        sector[108] = 9;
        sector[109] = 3;
        device.write_sector(0, &sector).unwrap();

        let probe = probe_filesystem(&mut device).unwrap();
        assert_eq!(probe.kind, FileSystemKind::ExFat);
        assert_eq!(
            probe.details,
            Some(FileSystemDetails::ExFat(ExFatInfo {
                bytes_per_sector: 512,
                sectors_per_cluster: 8,
                cluster_count: 256,
                root_cluster: 5,
            }))
        );
    }
}
