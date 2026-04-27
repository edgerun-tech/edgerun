use crate::prelude::v1::*;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::read_volatile;

use crate::types::TpmError;

const RSDP_SIGNATURE: &[u8; 8] = b"RSD PTR ";
const SDT_HEADER_SIZE: usize = 36;
const RSDP_V1_SIZE: usize = 20;
const RSDP_V2_MIN_SIZE: usize = 36;

const EBDA_SEGMENT_PTR: usize = 0x40e;
const BIOS_SEARCH_START: usize = 0x000e_0000;
const BIOS_SEARCH_END: usize = 0x0010_0000;

/// TPM details discovered from the ACPI `TPM2` table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AcpiTpm2Info {
    pub platform_class: u16,
    pub control_area: u64,
    pub start_method: u32,
}

impl AcpiTpm2Info {
    pub fn is_crb(&self) -> bool {
        matches!(self.start_method, 6 | 7 | 8) && self.control_area != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RsdpInfo {
    revision: u8,
    rsdt_address: u32,
    xsdt_address: Option<u64>,
}

/// Discover TPM2 ACPI information by scanning conventional x86 firmware areas.
///
/// # Safety
///
/// This assumes physical memory is directly readable at its physical address,
/// which is true for the current simple unikernel boot model but must be
/// revisited once paging stops identity-mapping firmware tables.
pub unsafe fn discover_tpm2_info() -> Result<Option<AcpiTpm2Info>, TpmError> {
    let Some(rsdp_addr) = (unsafe { find_rsdp_physical() }) else {
        return Ok(None);
    };
    let rsdp = unsafe { read_physical(rsdp_addr, RSDP_V2_MIN_SIZE) };
    let rsdp = parse_rsdp(&rsdp)?;

    if let Some(xsdt) = rsdp.xsdt_address {
        if let Some(info) = unsafe { find_tpm2_in_sdt(xsdt as usize, true) }? {
            return Ok(Some(info));
        }
    }

    if rsdp.rsdt_address != 0 {
        return unsafe { find_tpm2_in_sdt(rsdp.rsdt_address as usize, false) };
    }

    Ok(None)
}

pub fn parse_tpm2_table(table: &[u8]) -> Result<AcpiTpm2Info, TpmError> {
    validate_sdt(table, b"TPM2")?;
    if table.len() < 52 {
        return Err(TpmError::Protocol("ACPI TPM2 table too short".into()));
    }

    Ok(AcpiTpm2Info {
        platform_class: read_le_u16(table, 36)?,
        control_area: read_le_u64(table, 40)?,
        start_method: read_le_u32(table, 48)?,
    })
}

fn parse_rsdp(bytes: &[u8]) -> Result<RsdpInfo, TpmError> {
    if bytes.len() < RSDP_V1_SIZE {
        return Err(TpmError::Protocol("ACPI RSDP too short".into()));
    }
    if &bytes[..8] != RSDP_SIGNATURE {
        return Err(TpmError::Protocol("ACPI RSDP signature mismatch".into()));
    }
    if checksum(&bytes[..RSDP_V1_SIZE]) != 0 {
        return Err(TpmError::Protocol("ACPI RSDP checksum failed".into()));
    }

    let revision = bytes[15];
    let rsdt_address = read_le_u32(bytes, 16)?;
    let xsdt_address = if revision >= 2 && bytes.len() >= RSDP_V2_MIN_SIZE {
        let length = read_le_u32(bytes, 20)? as usize;
        if length >= RSDP_V2_MIN_SIZE && bytes.len() >= length && checksum(&bytes[..length]) == 0 {
            let xsdt = read_le_u64(bytes, 24)?;
            (xsdt != 0).then_some(xsdt)
        } else {
            None
        }
    } else {
        None
    };

    Ok(RsdpInfo {
        revision,
        rsdt_address,
        xsdt_address,
    })
}

fn validate_sdt(table: &[u8], signature: &[u8; 4]) -> Result<(), TpmError> {
    if table.len() < SDT_HEADER_SIZE {
        return Err(TpmError::Protocol("ACPI SDT header too short".into()));
    }
    if &table[..4] != signature {
        return Err(TpmError::Protocol("ACPI SDT signature mismatch".into()));
    }
    let length = read_le_u32(table, 4)? as usize;
    if length != table.len() {
        return Err(TpmError::Protocol("ACPI SDT length mismatch".into()));
    }
    if checksum(table) != 0 {
        return Err(TpmError::Protocol("ACPI SDT checksum failed".into()));
    }
    Ok(())
}

unsafe fn find_tpm2_in_sdt(address: usize, xsdt: bool) -> Result<Option<AcpiTpm2Info>, TpmError> {
    let header = unsafe { read_physical(address, SDT_HEADER_SIZE) };
    if header.len() < SDT_HEADER_SIZE {
        return Ok(None);
    }
    let length = read_le_u32(&header, 4)? as usize;
    if length < SDT_HEADER_SIZE {
        return Ok(None);
    }
    let table = unsafe { read_physical(address, length) };
    let signature = if xsdt { b"XSDT" } else { b"RSDT" };
    validate_sdt(&table, signature)?;

    let entry_size = if xsdt { 8 } else { 4 };
    let mut cursor = SDT_HEADER_SIZE;
    while cursor + entry_size <= table.len() {
        let entry = if xsdt {
            read_le_u64(&table, cursor)? as usize
        } else {
            read_le_u32(&table, cursor)? as usize
        };
        cursor += entry_size;
        if entry == 0 {
            continue;
        }

        let child_header = unsafe { read_physical(entry, SDT_HEADER_SIZE) };
        if child_header.len() >= 4 && &child_header[..4] == b"TPM2" {
            let child_len = read_le_u32(&child_header, 4)? as usize;
            let child = unsafe { read_physical(entry, child_len) };
            return parse_tpm2_table(&child).map(Some);
        }
    }

    Ok(None)
}

unsafe fn find_rsdp_physical() -> Option<usize> {
    let ebda_segment = unsafe { read_u16_phys(EBDA_SEGMENT_PTR) } as usize;
    let ebda_base = ebda_segment << 4;
    if ebda_base != 0 {
        if let Some(found) = unsafe { scan_rsdp_range(ebda_base, ebda_base.saturating_add(1024)) } {
            return Some(found);
        }
    }

    unsafe { scan_rsdp_range(BIOS_SEARCH_START, BIOS_SEARCH_END) }
}

unsafe fn scan_rsdp_range(start: usize, end: usize) -> Option<usize> {
    let mut address = start;
    while address + RSDP_V1_SIZE <= end {
        if unsafe { physical_starts_with(address, RSDP_SIGNATURE) } {
            return Some(address);
        }
        address = address.saturating_add(16);
    }
    None
}

unsafe fn physical_starts_with(address: usize, expected: &[u8]) -> bool {
    for (offset, expected_byte) in expected.iter().enumerate() {
        let byte = unsafe { read_volatile((address + offset) as *const u8) };
        if byte != *expected_byte {
            return false;
        }
    }
    true
}

unsafe fn read_physical(address: usize, len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    for (offset, byte) in out.iter_mut().enumerate() {
        *byte = unsafe { read_volatile((address + offset) as *const u8) };
    }
    out
}

unsafe fn read_u16_phys(address: usize) -> u16 {
    let low = unsafe { read_volatile(address as *const u8) };
    let high = unsafe { read_volatile((address + 1) as *const u8) };
    u16::from_le_bytes([low, high])
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |acc, byte| acc.wrapping_add(*byte))
}

fn read_le_u16(bytes: &[u8], offset: usize) -> Result<u16, TpmError> {
    if bytes.len().saturating_sub(offset) < 2 {
        return Err(TpmError::Protocol("ACPI u16 exceeds buffer".into()));
    }
    Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32, TpmError> {
    if bytes.len().saturating_sub(offset) < 4 {
        return Err(TpmError::Protocol("ACPI u32 exceeds buffer".into()));
    }
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

fn read_le_u64(bytes: &[u8], offset: usize) -> Result<u64, TpmError> {
    if bytes.len().saturating_sub(offset) < 8 {
        return Err(TpmError::Protocol("ACPI u64 exceeds buffer".into()));
    }
    Ok(u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix_checksum(bytes: &mut [u8], checksum_offset: usize) {
        bytes[checksum_offset] = 0;
        let sum = checksum(bytes);
        bytes[checksum_offset] = 0u8.wrapping_sub(sum);
    }

    fn sdt(signature: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let len = SDT_HEADER_SIZE + body.len();
        let mut table = vec![0u8; len];
        table[..4].copy_from_slice(signature);
        table[4..8].copy_from_slice(&(len as u32).to_le_bytes());
        table[8] = 1;
        table[10..16].copy_from_slice(b"EDGERN");
        table[16..24].copy_from_slice(b"TPMTEST ");
        table[24..28].copy_from_slice(&1u32.to_le_bytes());
        table[28..32].copy_from_slice(b"EDGE");
        table[32..36].copy_from_slice(&1u32.to_le_bytes());
        table[SDT_HEADER_SIZE..].copy_from_slice(body);
        fix_checksum(&mut table, 9);
        table
    }

    #[test]
    fn parse_tpm2_table_reads_crb_fields() {
        let mut body = Vec::new();
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0x0000_0000_fed4_0000u64.to_le_bytes());
        body.extend_from_slice(&6u32.to_le_bytes());
        let table = sdt(b"TPM2", &body);

        let info = parse_tpm2_table(&table).unwrap();
        assert_eq!(info.platform_class, 0);
        assert_eq!(info.control_area, 0xfed4_0000);
        assert_eq!(info.start_method, 6);
        assert!(info.is_crb());
    }

    #[test]
    fn parse_tpm2_table_rejects_bad_checksum() {
        let body = vec![0u8; 16];
        let mut table = sdt(b"TPM2", &body);
        table[9] = table[9].wrapping_add(1);
        assert!(parse_tpm2_table(&table).is_err());
    }

    #[test]
    fn parse_rsdp_prefers_valid_xsdt() {
        let mut rsdp = vec![0u8; RSDP_V2_MIN_SIZE];
        rsdp[..8].copy_from_slice(RSDP_SIGNATURE);
        rsdp[8..14].copy_from_slice(b"EDGERN");
        rsdp[15] = 2;
        rsdp[16..20].copy_from_slice(&0x1234_0000u32.to_le_bytes());
        rsdp[20..24].copy_from_slice(&(RSDP_V2_MIN_SIZE as u32).to_le_bytes());
        rsdp[24..32].copy_from_slice(&0x0000_0001_2345_0000u64.to_le_bytes());
        fix_checksum(&mut rsdp[..RSDP_V1_SIZE], 8);
        fix_checksum(&mut rsdp, 32);

        let info = parse_rsdp(&rsdp).unwrap();
        assert_eq!(info.revision, 2);
        assert_eq!(info.rsdt_address, 0x1234_0000);
        assert_eq!(info.xsdt_address, Some(0x0000_0001_2345_0000));
    }
}
