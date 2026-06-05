//! Protocol-owned HPACK adapter boundary for HTTP/2.
//!
//! The retired compatibility crate is not restored here. This module owns
//! connection HPACK state for HTTP/2 runtimes. Compact WAT kernels scan header
//! block bytes, strings, Huffman payloads, prefix integers, and table arithmetic;
//! runtime code resolves static/dynamic table entries and validates HTTP
//! request or response semantics.

use alloc::vec::Vec;

const DEFAULT_DYNAMIC_TABLE_SIZE: usize = 4096;
const WAT_HEADER_RECORD_SIZE: usize = 48;

/// Decoded HTTP header list owned by the protocol runtime.
pub type HpackHeaderList = Vec<(Vec<u8>, Vec<u8>)>;

/// Connection-local HPACK context.
#[derive(Debug, Clone)]
pub struct HpackContext {
    max_dynamic_table_size: usize,
    dynamic_table_size: usize,
    dynamic_entries: Vec<DynamicEntry>,
}

impl HpackContext {
    /// Create an empty HTTP/2 HPACK context.
    pub fn new() -> Self {
        Self {
            max_dynamic_table_size: DEFAULT_DYNAMIC_TABLE_SIZE,
            dynamic_table_size: 0,
            dynamic_entries: Vec::new(),
        }
    }

    /// Decode one complete HPACK header block.
    pub fn decode_header_block(
        &mut self,
        _header_block: &[u8],
    ) -> Result<HpackHeaderList, HpackDecodeError> {
        Err(HpackDecodeError::NotYetRouted)
    }

    /// Encode one complete HPACK header block.
    ///
    /// This is a deterministic direct HPACK emission path for the current
    /// server response use case. It deliberately avoids dynamic-table insertion
    /// and Huffman encoding until the runtime invokes `http-prefix-int.wat` and
    /// `hpack-string.wat` directly.
    pub fn encode_header_block<'a, I>(&mut self, headers: I) -> Result<Vec<u8>, HpackEncodeError>
    where
        I: IntoIterator<Item = (&'a [u8], &'a [u8])>,
    {
        let mut out = Vec::new();
        for (name, value) in headers {
            if name.is_empty() {
                return Err(HpackEncodeError::InvalidHeader);
            }
            if name == b":status" && value == b"200" {
                encode_indexed_header(8, &mut out)?;
                continue;
            }

            match static_name_index(name) {
                Some(index) => {
                    encode_never_indexed_literal_indexed_name(index, value, &mut out)?;
                }
                None => {
                    encode_never_indexed_literal_new_name(name, value, &mut out)?;
                }
            }
        }
        Ok(out)
    }

    /// Dynamic table size accepted by this connection context.
    pub const fn max_dynamic_table_size(&self) -> usize {
        self.max_dynamic_table_size
    }

    /// Current dynamic table byte size tracked by this connection context.
    pub const fn dynamic_table_size(&self) -> usize {
        self.dynamic_table_size
    }

    /// Apply an HTTP/2 `SETTINGS_HEADER_TABLE_SIZE` value to this HPACK
    /// context.
    ///
    /// Runtime code owns the table entries. `hpack-table-core.wat` owns the
    /// arithmetic record that future host plumbing should call before this
    /// method mutates the in-memory table.
    pub fn set_max_table_size(&mut self, max_size: usize) {
        self.max_dynamic_table_size = max_size;
        self.evict_to_size(max_size);
    }

    /// Apply WAT-scanned header instruction records to this connection context.
    ///
    /// The host runtime must call:
    ///
    /// - `hpack-header-block.wat::hpack_header_block_scan`
    /// - `hpack-string.wat::hpack_string_decode` for Huffman string payloads
    /// - `hpack-table-core.wat` for insert/resize plans before table mutation
    ///
    /// This method maps the resulting record stream into protocol-owned header
    /// records. It handles static indexed fields and raw literal strings. It
    /// rejects Huffman literals until the host passes decoded string bytes
    /// through an explicit decoded-string record.
    pub fn decode_wat_records(
        &mut self,
        header_block: &[u8],
        records: &[HpackWatHeaderInstruction],
    ) -> Result<HpackHeaderList, HpackDecodeError> {
        let mut headers = Vec::new();
        for record in records {
            match record.kind {
                HpackWatInstructionKind::IndexedHeader => {
                    let (name, value) = self
                        .resolve_index(record.value as usize)
                        .ok_or(HpackDecodeError::InvalidTableIndex)?;
                    headers.push((name.to_vec(), value.to_vec()));
                }
                HpackWatInstructionKind::LiteralIndexedName
                | HpackWatInstructionKind::LiteralNewName
                | HpackWatInstructionKind::NeverIndexed => {
                    let name = if record.value == 0 {
                        copy_raw_span(
                            header_block,
                            record.name_payload_offset,
                            record.name_payload_len,
                            record.name_huffman,
                        )?
                    } else {
                        self.resolve_index(record.value as usize)
                            .ok_or(HpackDecodeError::InvalidTableIndex)?
                            .0
                            .to_vec()
                    };
                    let value = copy_raw_span(
                        header_block,
                        record.value_payload_offset,
                        record.value_payload_len,
                        record.value_huffman,
                    )?;
                    if matches!(
                        record.kind,
                        HpackWatInstructionKind::LiteralIndexedName
                            | HpackWatInstructionKind::LiteralNewName
                    ) {
                        self.insert_dynamic(name.clone(), value.clone());
                    }
                    headers.push((name, value));
                }
                HpackWatInstructionKind::DynamicTableSizeUpdate => {
                    let size = usize::try_from(record.value)
                        .map_err(|_| HpackDecodeError::DynamicTableSizeExceeded)?;
                    if size > self.max_dynamic_table_size {
                        return Err(HpackDecodeError::DynamicTableSizeExceeded);
                    }
                    self.evict_to_size(size);
                }
            }
        }
        Ok(headers)
    }

    fn resolve_index(&self, index: usize) -> Option<(&[u8], &[u8])> {
        if index == 0 {
            return None;
        }
        if let Some(entry) = STATIC_TABLE.get(index - 1) {
            return Some(*entry);
        }
        let dynamic_index = index.checked_sub(STATIC_TABLE.len() + 1)?;
        self.dynamic_entries
            .get(dynamic_index)
            .map(|entry| (entry.name.as_slice(), entry.value.as_slice()))
    }

    fn insert_dynamic(&mut self, name: Vec<u8>, value: Vec<u8>) {
        let size = hpack_entry_size(name.len(), value.len());
        if size > self.max_dynamic_table_size {
            self.dynamic_entries.clear();
            self.dynamic_table_size = 0;
            return;
        }
        self.dynamic_entries
            .insert(0, DynamicEntry { name, value, size });
        self.dynamic_table_size = self.dynamic_table_size.saturating_add(size);
        self.evict_to_size(self.max_dynamic_table_size);
    }

    fn evict_to_size(&mut self, max_size: usize) {
        while self.dynamic_table_size > max_size {
            match self.dynamic_entries.pop() {
                Some(entry) => {
                    self.dynamic_table_size = self.dynamic_table_size.saturating_sub(entry.size);
                }
                None => {
                    self.dynamic_table_size = 0;
                    break;
                }
            }
        }
    }
}

impl Default for HpackContext {
    fn default() -> Self {
        Self::new()
    }
}

/// HPACK decode failure mapped by HTTP/2 callers to `COMPRESSION_ERROR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HpackDecodeError {
    /// WAT invocation plumbing has not been connected to this runtime path yet.
    NotYetRouted,
    /// Header-block bytes were malformed or incomplete.
    InvalidHeaderBlock,
    /// Header block referenced a static or dynamic table index that is invalid.
    InvalidTableIndex,
    /// A dynamic table update exceeded the SETTINGS-bound maximum.
    DynamicTableSizeExceeded,
    /// A scanned string requires a WAT Huffman/string decode result that was
    /// not supplied to this runtime boundary.
    HuffmanStringNotDecoded,
}

/// HPACK encode failure mapped by HTTP/2 callers to `COMPRESSION_ERROR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HpackEncodeError {
    /// WAT invocation plumbing has not been connected to this runtime path yet.
    NotYetRouted,
    /// Header name or value cannot be represented as an HPACK literal.
    InvalidHeader,
    /// Header block would exceed the active output limit.
    OutputLimitExceeded,
}

/// Status values returned by the WAT HPACK byte kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum HpackWatStatus {
    Ok = 0,
    Truncated = 1,
    OutputTooSmall = 2,
    Invalid = 3,
    Overflow = 4,
}

impl HpackWatStatus {
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Ok),
            1 => Some(Self::Truncated),
            2 => Some(Self::OutputTooSmall),
            3 => Some(Self::Invalid),
            4 => Some(Self::Overflow),
            _ => None,
        }
    }
}

/// Instruction kinds emitted by `hpack-header-block.wat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum HpackWatInstructionKind {
    IndexedHeader = 1,
    LiteralIndexedName = 2,
    LiteralNewName = 3,
    NeverIndexed = 4,
    DynamicTableSizeUpdate = 5,
}

impl HpackWatInstructionKind {
    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::IndexedHeader),
            2 => Some(Self::LiteralIndexedName),
            3 => Some(Self::LiteralNewName),
            4 => Some(Self::NeverIndexed),
            5 => Some(Self::DynamicTableSizeUpdate),
            _ => None,
        }
    }
}

/// Host-decoded form of one 48-byte `hpack-header-block.wat` record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HpackWatHeaderInstruction {
    pub kind: HpackWatInstructionKind,
    pub offset: u32,
    pub consumed: u32,
    pub flags: u32,
    pub value: u64,
    pub name_payload_offset: u32,
    pub name_payload_len: u32,
    pub name_huffman: bool,
    pub value_payload_offset: u32,
    pub value_payload_len: u32,
    pub value_huffman: bool,
}

impl HpackWatHeaderInstruction {
    /// Decode the 48-byte little-endian WAT record layout.
    pub fn from_wat_record(bytes: &[u8]) -> Result<Self, HpackDecodeError> {
        if bytes.len() < WAT_HEADER_RECORD_SIZE {
            return Err(HpackDecodeError::InvalidHeaderBlock);
        }
        let kind = HpackWatInstructionKind::from_u32(read_u32_le(bytes, 0))
            .ok_or(HpackDecodeError::InvalidHeaderBlock)?;
        Ok(Self {
            kind,
            offset: read_u32_le(bytes, 4),
            consumed: read_u32_le(bytes, 8),
            flags: read_u32_le(bytes, 12),
            value: read_u64_le(bytes, 16),
            name_payload_offset: read_u32_le(bytes, 24),
            name_payload_len: read_u32_le(bytes, 28),
            name_huffman: read_u32_le(bytes, 32) != 0,
            value_payload_offset: read_u32_le(bytes, 36),
            value_payload_len: read_u32_le(bytes, 40),
            value_huffman: read_u32_le(bytes, 44) != 0,
        })
    }
}

#[derive(Debug, Clone)]
struct DynamicEntry {
    name: Vec<u8>,
    value: Vec<u8>,
    size: usize,
}

fn copy_raw_span(
    source: &[u8],
    offset: u32,
    len: u32,
    huffman: bool,
) -> Result<Vec<u8>, HpackDecodeError> {
    if huffman {
        return Err(HpackDecodeError::HuffmanStringNotDecoded);
    }
    let start = usize::try_from(offset).map_err(|_| HpackDecodeError::InvalidHeaderBlock)?;
    let len = usize::try_from(len).map_err(|_| HpackDecodeError::InvalidHeaderBlock)?;
    let end = start
        .checked_add(len)
        .ok_or(HpackDecodeError::InvalidHeaderBlock)?;
    let span = source
        .get(start..end)
        .ok_or(HpackDecodeError::InvalidHeaderBlock)?;
    Ok(span.to_vec())
}

fn encode_indexed_header(index: u64, out: &mut Vec<u8>) -> Result<(), HpackEncodeError> {
    encode_prefixed_integer(index, 7, 0b1, out)
}

fn encode_never_indexed_literal_indexed_name(
    name_index: u64,
    value: &[u8],
    out: &mut Vec<u8>,
) -> Result<(), HpackEncodeError> {
    encode_prefixed_integer(name_index, 4, 0b1, out)?;
    encode_raw_string(value, out)
}

fn encode_never_indexed_literal_new_name(
    name: &[u8],
    value: &[u8],
    out: &mut Vec<u8>,
) -> Result<(), HpackEncodeError> {
    encode_prefixed_integer(0, 4, 0b1, out)?;
    encode_raw_string(name, out)?;
    encode_raw_string(value, out)
}

fn encode_raw_string(bytes: &[u8], out: &mut Vec<u8>) -> Result<(), HpackEncodeError> {
    encode_prefixed_integer(
        u64::try_from(bytes.len()).map_err(|_| HpackEncodeError::OutputLimitExceeded)?,
        7,
        0,
        out,
    )?;
    out.extend_from_slice(bytes);
    Ok(())
}

fn encode_prefixed_integer(
    value: u64,
    prefix_bits: u8,
    high_bits: u8,
    out: &mut Vec<u8>,
) -> Result<(), HpackEncodeError> {
    if !(1..=8).contains(&prefix_bits) {
        return Err(HpackEncodeError::InvalidHeader);
    }
    let mask = if prefix_bits == 8 {
        0xff
    } else {
        (1u16 << prefix_bits) - 1
    };
    let max_high = if prefix_bits == 8 {
        1
    } else {
        1u16 << (8 - prefix_bits)
    };
    if u16::from(high_bits) >= max_high {
        return Err(HpackEncodeError::InvalidHeader);
    }
    let flags = u16::from(high_bits) << prefix_bits;
    if value < u64::from(mask) {
        out.push((flags | value as u16) as u8);
        return Ok(());
    }

    out.push((flags | mask) as u8);
    let mut remaining = value - u64::from(mask);
    loop {
        let mut byte = (remaining & 0x7f) as u8;
        remaining >>= 7;
        if remaining != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if remaining == 0 {
            break;
        }
    }
    Ok(())
}

fn hpack_entry_size(name_len: usize, value_len: usize) -> usize {
    name_len.saturating_add(value_len).saturating_add(32)
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u64_le(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

fn static_name_index(name: &[u8]) -> Option<u64> {
    STATIC_TABLE
        .iter()
        .position(|(static_name, _)| *static_name == name)
        .map(|index| (index + 1) as u64)
}

const STATIC_TABLE: &[(&[u8], &[u8])] = &[
    (b":authority", b""),
    (b":method", b"GET"),
    (b":method", b"POST"),
    (b":path", b"/"),
    (b":path", b"/index.html"),
    (b":scheme", b"http"),
    (b":scheme", b"https"),
    (b":status", b"200"),
    (b":status", b"204"),
    (b":status", b"206"),
    (b":status", b"304"),
    (b":status", b"400"),
    (b":status", b"404"),
    (b":status", b"500"),
    (b"accept-charset", b""),
    (b"accept-encoding", b"gzip, deflate"),
    (b"accept-language", b""),
    (b"accept-ranges", b""),
    (b"accept", b""),
    (b"access-control-allow-origin", b""),
    (b"age", b""),
    (b"allow", b""),
    (b"authorization", b""),
    (b"cache-control", b""),
    (b"content-disposition", b""),
    (b"content-encoding", b""),
    (b"content-language", b""),
    (b"content-length", b""),
    (b"content-location", b""),
    (b"content-range", b""),
    (b"content-type", b""),
    (b"cookie", b""),
    (b"date", b""),
    (b"etag", b""),
    (b"expect", b""),
    (b"expires", b""),
    (b"from", b""),
    (b"host", b""),
    (b"if-match", b""),
    (b"if-modified-since", b""),
    (b"if-none-match", b""),
    (b"if-range", b""),
    (b"if-unmodified-since", b""),
    (b"last-modified", b""),
    (b"link", b""),
    (b"location", b""),
    (b"max-forwards", b""),
    (b"proxy-authenticate", b""),
    (b"proxy-authorization", b""),
    (b"range", b""),
    (b"referer", b""),
    (b"refresh", b""),
    (b"retry-after", b""),
    (b"server", b""),
    (b"set-cookie", b""),
    (b"strict-transport-security", b""),
    (b"transfer-encoding", b""),
    (b"user-agent", b""),
    (b"vary", b""),
    (b"via", b""),
    (b"www-authenticate", b""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn deterministic_server_response_encode_uses_static_and_never_indexed_literals() {
        let mut hpack = HpackContext::new();
        let headers = [
            (&b":status"[..], &b"200"[..]),
            (&b"content-type"[..], &b"text/plain"[..]),
            (&b"content-length"[..], &b"2"[..]),
        ];

        let encoded = hpack.encode_header_block(headers).unwrap();

        assert_eq!(
            encoded,
            vec![
                0x88, // indexed static :status 200
                0x1f, 0x10, // never-indexed literal, static name 31
                0x0a, b't', b'e', b'x', b't', b'/', b'p', b'l', b'a', b'i', b'n', 0x1f,
                0x0d, // never-indexed literal, static name 28
                0x01, b'2',
            ]
        );
    }

    #[test]
    fn wat_record_decode_maps_static_and_raw_literal_headers() {
        let mut hpack = HpackContext::new();
        let header_block = [
            0x82, // indexed static :method GET
            0x10, 0x03, b'f', b'o', b'o', 0x03, b'b', b'a', b'r',
        ];
        let records = [
            HpackWatHeaderInstruction {
                kind: HpackWatInstructionKind::IndexedHeader,
                offset: 0,
                consumed: 1,
                flags: 1,
                value: 2,
                name_payload_offset: 0,
                name_payload_len: 0,
                name_huffman: false,
                value_payload_offset: 0,
                value_payload_len: 0,
                value_huffman: false,
            },
            HpackWatHeaderInstruction {
                kind: HpackWatInstructionKind::NeverIndexed,
                offset: 1,
                consumed: 9,
                flags: 1,
                value: 0,
                name_payload_offset: 3,
                name_payload_len: 3,
                name_huffman: false,
                value_payload_offset: 7,
                value_payload_len: 3,
                value_huffman: false,
            },
        ];

        let decoded = hpack.decode_wat_records(&header_block, &records).unwrap();

        assert_eq!(
            decoded,
            vec![
                (b":method".to_vec(), b"GET".to_vec()),
                (b"foo".to_vec(), b"bar".to_vec()),
            ]
        );
    }

    #[test]
    fn wat_record_boundary_decodes_little_endian_layout() {
        let mut bytes = [0u8; WAT_HEADER_RECORD_SIZE];
        bytes[0..4]
            .copy_from_slice(&(HpackWatInstructionKind::LiteralIndexedName as u32).to_le_bytes());
        bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&3u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
        bytes[16..24].copy_from_slice(&31u64.to_le_bytes());
        bytes[36..40].copy_from_slice(&2u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&1u32.to_le_bytes());

        let record = HpackWatHeaderInstruction::from_wat_record(&bytes).unwrap();

        assert_eq!(record.kind, HpackWatInstructionKind::LiteralIndexedName);
        assert_eq!(record.offset, 2);
        assert_eq!(record.consumed, 3);
        assert_eq!(record.value, 31);
        assert_eq!(record.value_payload_offset, 2);
        assert_eq!(record.value_payload_len, 1);
    }

    #[test]
    fn table_size_setting_evicts_runtime_dynamic_entries() {
        let mut hpack = HpackContext::new();
        let header_block = [0x40, 0x03, b'f', b'o', b'o', 0x03, b'b', b'a', b'r'];
        let records = [HpackWatHeaderInstruction {
            kind: HpackWatInstructionKind::LiteralNewName,
            offset: 0,
            consumed: 9,
            flags: 1,
            value: 0,
            name_payload_offset: 2,
            name_payload_len: 3,
            name_huffman: false,
            value_payload_offset: 6,
            value_payload_len: 3,
            value_huffman: false,
        }];

        let decoded = hpack.decode_wat_records(&header_block, &records).unwrap();

        assert_eq!(decoded, vec![(b"foo".to_vec(), b"bar".to_vec())]);
        assert_eq!(hpack.dynamic_table_size(), 38);
        hpack.set_max_table_size(0);
        assert_eq!(hpack.max_dynamic_table_size(), 0);
        assert_eq!(hpack.dynamic_table_size(), 0);
    }

    #[test]
    fn decode_header_block_stays_not_yet_routed_without_runtime_wasm_invocation() {
        let mut hpack = HpackContext::new();

        assert_eq!(
            hpack.decode_header_block(&[0x82]).unwrap_err(),
            HpackDecodeError::NotYetRouted
        );
    }
}
