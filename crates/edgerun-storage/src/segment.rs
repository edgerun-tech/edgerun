// SPDX-License-Identifier: Apache-2.0
use blake3::Hasher;
use std::collections::HashMap;
use thiserror::Error;

use crate::encryption::{self, SegmentEncryptionConfig};
use crate::event::{Event, EventError, HlcTimestamp, StreamId, SEG0_MAGIC, VERSION};
use crate::io_reactor::IoReactor;

#[derive(Error, Debug)]
pub enum SegmentError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not sealed")]
    NotSealed,
    #[error("Already sealed")]
    AlreadySealed,
    #[error("Full")]
    Full,
    #[error("Invalid segment")]
    InvalidSegment,
    #[error("Event error: {0}")]
    Event(#[from] EventError),
    #[error("Encryption error: {0}")]
    Encryption(#[from] encryption::EncryptionError),
}

#[derive(Debug, Clone, Default)]
pub struct SegmentHeader {
    pub magic: u32,
    pub version: u16,
    pub segment_id: [u8; 32],
    pub created_hlc: HlcTimestamp,
    pub min_hlc: Option<HlcTimestamp>,
    pub max_hlc: Option<HlcTimestamp>,
    pub record_count: u32,
    pub header_crc: u32,
}

#[derive(Debug, Clone, Default)]
pub struct StreamSummaryEntry {
    pub min_hlc: HlcTimestamp,
    pub max_hlc: HlcTimestamp,
    pub count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct StreamSummary {
    pub streams: HashMap<StreamId, StreamSummaryEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct SegmentFooter {
    pub record_offsets: Vec<u64>,
    pub stream_summary: StreamSummary,
    pub merkle_root: [u8; 32],
    pub segment_hash: [u8; 32],
    pub footer_crc: u32,
}

pub struct Segment {
    path: std::path::PathBuf,
    header: SegmentHeader,
    footer: SegmentFooter,
    is_sealed: bool,
    max_size: u64,
    current_offset: u64,
    data: Vec<u8>,
    encryption: Option<SegmentEncryptionConfig>,
}

impl Segment {
    pub fn new(path: std::path::PathBuf, max_size: u64) -> Self {
        Self {
            path,
            header: SegmentHeader {
                magic: SEG0_MAGIC,
                version: VERSION,
                segment_id: [0u8; 32],
                created_hlc: HlcTimestamp::now(),
                min_hlc: None,
                max_hlc: None,
                record_count: 0,
                header_crc: 0,
            },
            footer: SegmentFooter::default(),
            is_sealed: false,
            max_size,
            current_offset: 0,
            data: Vec::new(),
            encryption: None,
        }
    }

    pub fn enable_encryption(&mut self, config: SegmentEncryptionConfig) {
        self.encryption = Some(config);
    }

    pub fn append_event(&mut self, event: &Event) -> Result<u64, SegmentError> {
        if self.is_sealed {
            return Err(SegmentError::AlreadySealed);
        }

        let serialized = event.serialize()?;
        self.append_serialized_event(event, &serialized)
    }

    pub fn append_serialized_event(
        &mut self,
        event: &Event,
        serialized: &[u8],
    ) -> Result<u64, SegmentError> {
        self.append_serialized(serialized, event.hlc_timestamp)
    }

    pub fn append_serialized(
        &mut self,
        serialized: &[u8],
        hlc: HlcTimestamp,
    ) -> Result<u64, SegmentError> {
        if self.is_sealed {
            return Err(SegmentError::AlreadySealed);
        }

        let record_offset = self.current_offset;

        if self.current_offset + serialized.len() as u64 > self.max_size {
            return Err(SegmentError::Full);
        }

        self.data.extend_from_slice(serialized);
        self.current_offset += serialized.len() as u64;

        self.header.record_count += 1;

        self.header.min_hlc = Some(self.header.min_hlc.map_or(hlc, |min| min.min(hlc)));
        self.header.max_hlc = Some(self.header.max_hlc.map_or(hlc, |max| max.max(hlc)));

        self.footer.record_offsets.push(record_offset);

        Ok(record_offset)
    }

    pub fn seal(&mut self) -> Result<[u8; 32], SegmentError> {
        if self.is_sealed {
            return Err(SegmentError::AlreadySealed);
        }

        let segment_hash = self.compute_segment_hash();
        self.header.segment_id = segment_hash;
        self.footer.segment_hash = segment_hash;

        let merkle_root = self.compute_merkle_root();
        self.footer.merkle_root = merkle_root;

        self.is_sealed = true;

        Ok(segment_hash)
    }

    /// Hash used for compute identity validation.
    /// This is strictly the payload bytes and excludes storage-layer metadata.
    pub fn bundle_hash(&self) -> [u8; 32] {
        edgerun_crypto::compute_bundle_hash(&self.data)
    }

    /// Hash used for storage/event-sourcing identity.
    /// This includes storage metadata such as `created_hlc`.
    pub fn segment_hash(&self) -> [u8; 32] {
        if self.is_sealed && self.footer.segment_hash != [0u8; 32] {
            self.footer.segment_hash
        } else {
            self.compute_segment_hash()
        }
    }

    /// Raw payload bytes for bundle hashing and deterministic compute identity.
    pub fn bundle_payload_bytes(&self) -> &[u8] {
        &self.data
    }

    fn compute_segment_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(&self.header.magic.to_le_bytes());
        hasher.update(&self.header.version.to_le_bytes());
        hasher.update(&self.header.created_hlc.to_bytes());
        hasher.update(&self.data);
        *hasher.finalize().as_bytes()
    }

    fn compute_merkle_root(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();

        let chunk_size = 4096;
        let mut offset = 0;
        while offset < self.data.len() {
            let end = std::cmp::min(offset + chunk_size, self.data.len());
            let chunk_hash = blake3::hash(&self.data[offset..end]);
            hasher.update(chunk_hash.as_bytes());
            offset = end;
        }

        *hasher.finalize().as_bytes()
    }

    pub fn is_sealed(&self) -> bool {
        self.is_sealed
    }

    pub fn segment_id(&self) -> [u8; 32] {
        self.header.segment_id
    }

    pub fn record_count(&self) -> u32 {
        self.header.record_count
    }

    pub fn min_hlc(&self) -> Option<HlcTimestamp> {
        self.header.min_hlc
    }

    pub fn max_hlc(&self) -> Option<HlcTimestamp> {
        self.header.max_hlc
    }

    pub fn merkle_root(&self) -> [u8; 32] {
        self.footer.merkle_root
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn get_event_at(&self, offset: u64) -> Result<Event, SegmentError> {
        if offset as usize >= self.data.len() {
            return Err(SegmentError::InvalidSegment);
        }

        let event = Event::deserialize(&self.data[offset as usize..])?;
        Ok(event)
    }

    /// Get current write offset.
    pub fn current_offset(&self) -> u64 {
        self.current_offset
    }

    pub fn iter_events(&self) -> SegmentEventIterator<'_> {
        SegmentEventIterator {
            segment: self,
            offset: 0,
        }
    }

    /// Serialize segment to bytes (used for final sealing/checkpoint snapshots).
    pub fn serialize_result(&self) -> Result<Vec<u8>, SegmentError> {
        if let Some(cfg) = &self.encryption {
            // Avoid materializing one giant plaintext buffer in encrypted mode.
            let mut header_bytes = Vec::with_capacity(42);
            header_bytes.extend_from_slice(&self.header.magic.to_le_bytes());
            header_bytes.extend_from_slice(&self.header.version.to_le_bytes());
            header_bytes.extend_from_slice(&self.header.record_count.to_le_bytes());
            header_bytes.extend_from_slice(&self.header.segment_id);

            let mut offsets_bytes = Vec::with_capacity(self.footer.record_offsets.len() * 8);
            for offset in &self.footer.record_offsets {
                offsets_bytes.extend_from_slice(&offset.to_le_bytes());
            }

            let mut tail_bytes = Vec::with_capacity(72);
            tail_bytes.extend_from_slice(&self.footer.merkle_root);
            tail_bytes.extend_from_slice(&self.footer.segment_hash);
            let offsets_count = self.footer.record_offsets.len() as u64;
            tail_bytes.extend_from_slice(&offsets_count.to_le_bytes());

            let parts: [&[u8]; 4] = [&header_bytes, &self.data, &offsets_bytes, &tail_bytes];
            let plaintext_len =
                header_bytes.len() + self.data.len() + offsets_bytes.len() + tail_bytes.len();
            return Ok(encryption::encrypt_segment_parts(
                &parts,
                plaintext_len,
                cfg,
            )?);
        }

        let mut file_data = Vec::new();

        // Header (42 bytes)
        file_data.extend_from_slice(&self.header.magic.to_le_bytes());
        file_data.extend_from_slice(&self.header.version.to_le_bytes());
        file_data.extend_from_slice(&self.header.record_count.to_le_bytes());
        file_data.extend_from_slice(&self.header.segment_id);

        // Data
        file_data.extend_from_slice(&self.data);

        // Footer - record offsets first
        for offset in &self.footer.record_offsets {
            file_data.extend_from_slice(&offset.to_le_bytes());
        }

        // Then merkle_root, segment_hash, and offsets_count at the end
        file_data.extend_from_slice(&self.footer.merkle_root);
        file_data.extend_from_slice(&self.footer.segment_hash);
        let offsets_count = self.footer.record_offsets.len() as u64;
        file_data.extend_from_slice(&offsets_count.to_le_bytes());
        Ok(file_data)
    }

    /// Infallible convenience wrapper retained for existing callers/tests.
    pub fn serialize(&self) -> Vec<u8> {
        self.serialize_result().expect("segment serialize failed")
    }

    pub fn flush(&mut self) -> Result<(), SegmentError> {
        let file_data = self.serialize_result()?;

        if let Ok(reactor) = IoReactor::global() {
            let file = reactor.open_file(&self.path, true, true, true, true)?;
            reactor.truncate(file, 0).wait()?;
            reactor.write(file, 0, file_data).wait()?;
            reactor.fsync(file, false).wait()?;
            reactor.close(file);
            return Ok(());
        }

        // Fallback for environments without io_uring support.
        std::fs::write(&self.path, &file_data)?;
        Ok(())
    }

    pub fn sync(&self) -> Result<(), SegmentError> {
        if let Ok(reactor) = IoReactor::global() {
            let file = reactor.open_file(&self.path, false, true, true, false)?;
            reactor.fsync(file, false).wait()?;
            reactor.close(file);
            return Ok(());
        }

        use std::fs::File;
        let file = File::open(&self.path)?;
        file.sync_all()?;
        Ok(())
    }

    pub fn from_file(path: std::path::PathBuf) -> Result<Self, SegmentError> {
        let data = std::fs::read(&path)?;
        Self::from_bytes(path, data)
    }

    pub fn from_encrypted_file(
        path: std::path::PathBuf,
        store_key: [u8; 32],
        store_uuid: [u8; 16],
    ) -> Result<Self, SegmentError> {
        let data = std::fs::read(&path)?;
        let plain = encryption::decrypt_segment_bytes(&data, store_key, store_uuid)?;
        Self::from_bytes(path, plain)
    }

    fn from_bytes(path: std::path::PathBuf, data: Vec<u8>) -> Result<Self, SegmentError> {
        if data.len() < 4 {
            return Err(SegmentError::InvalidSegment);
        }
        let prefix_magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if prefix_magic == encryption::ENC_SEG_MAGIC {
            return Err(SegmentError::InvalidSegment);
        }

        if data.len() < 42 + 8 + 64 {
            return Err(SegmentError::InvalidSegment);
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != SEG0_MAGIC {
            return Err(SegmentError::InvalidSegment);
        }

        let version = u16::from_le_bytes([data[4], data[5]]);
        if version != VERSION {
            return Err(SegmentError::InvalidSegment);
        }

        let record_count = u32::from_le_bytes([data[6], data[7], data[8], data[9]]);

        let mut segment_id = [0u8; 32];
        segment_id.copy_from_slice(&data[10..42]);

        // Footer is at the end of the file
        // Read backwards from end:
        // - segment_hash: 32 bytes
        // - merkle_root: 32 bytes
        // - offsets_count: 8 bytes
        // - offsets: offsets_count * 8 bytes

        let file_len = data.len();

        // Check if this is a new format file with footer
        // New format: header (42) + data + offsets (record_count * 8) + footer fixed (72)
        let expected_size_with_footer = 42 + (record_count as usize * 8) + 72;
        let has_footer = file_len >= expected_size_with_footer;

        let (record_offsets, merkle_root, segment_hash, actual_data) = if has_footer {
            let footer_start = file_len - 72; // 32 + 32 + 8 for footer fixed part

            let mut merkle_root = [0u8; 32];
            merkle_root.copy_from_slice(&data[footer_start..footer_start + 32]);

            let mut segment_hash = [0u8; 32];
            segment_hash.copy_from_slice(&data[footer_start + 32..footer_start + 64]);

            let offsets_count = u64::from_le_bytes([
                data[footer_start + 64],
                data[footer_start + 65],
                data[footer_start + 66],
                data[footer_start + 67],
                data[footer_start + 68],
                data[footer_start + 69],
                data[footer_start + 70],
                data[footer_start + 71],
            ]) as usize;

            // Validate offsets_count matches record_count
            if offsets_count != record_count as usize {
                return Err(SegmentError::InvalidSegment);
            }

            let mut record_offsets: Vec<u64> = Vec::with_capacity(offsets_count);
            let offsets_start = footer_start - (offsets_count * 8);

            for i in 0..offsets_count {
                let offset_pos = offsets_start + (i * 8);
                let offset = u64::from_le_bytes([
                    data[offset_pos],
                    data[offset_pos + 1],
                    data[offset_pos + 2],
                    data[offset_pos + 3],
                    data[offset_pos + 4],
                    data[offset_pos + 5],
                    data[offset_pos + 6],
                    data[offset_pos + 7],
                ]);
                record_offsets.push(offset);
            }

            let actual_data = data[42..offsets_start].to_vec();
            (record_offsets, merkle_root, segment_hash, actual_data)
        } else {
            // Old format - no footer, data goes from byte 42 to end
            let actual_data = data[42..].to_vec();
            (Vec::new(), [0u8; 32], [0u8; 32], actual_data)
        };

        Ok(Self {
            path,
            header: SegmentHeader {
                magic: SEG0_MAGIC,
                version: VERSION,
                segment_id,
                created_hlc: HlcTimestamp::now(),
                min_hlc: None,
                max_hlc: None,
                record_count,
                header_crc: 0,
            },
            footer: SegmentFooter {
                record_offsets,
                stream_summary: StreamSummary::default(),
                merkle_root,
                segment_hash,
                footer_crc: 0,
            },
            is_sealed: true,
            max_size: 0,
            current_offset: actual_data.len() as u64,
            data: actual_data,
            encryption: None,
        })
    }
}

pub struct SegmentEventIterator<'a> {
    segment: &'a Segment,
    offset: u64,
}

impl<'a> Iterator for SegmentEventIterator<'a> {
    type Item = Result<Event, SegmentError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset as usize >= self.segment.data.len() {
            return None;
        }

        match Event::deserialize(&self.segment.data[self.offset as usize..]) {
            Ok(event) => {
                let event_size = event.serialize().map(|s| s.len()).unwrap_or(0) as u64;
                self.offset += event_size;
                Some(Ok(event))
            }
            Err(e) => Some(Err(SegmentError::Event(e))),
        }
    }
}

pub struct SegmentWriter {
    segment: Segment,
}

impl SegmentWriter {
    pub fn new(path: std::path::PathBuf, max_size: u64) -> Self {
        Self {
            segment: Segment::new(path, max_size),
        }
    }

    pub fn append(&mut self, event: &Event) -> Result<u64, SegmentError> {
        self.segment.append_event(event)
    }

    pub fn enable_encryption(&mut self, config: SegmentEncryptionConfig) {
        self.segment.enable_encryption(config);
    }

    pub fn seal(&mut self) -> Result<[u8; 32], SegmentError> {
        self.segment.seal()
    }

    pub fn is_sealed(&self) -> bool {
        self.segment.is_sealed()
    }

    pub fn segment_id(&self) -> [u8; 32] {
        self.segment.segment_id()
    }

    pub fn segment_hash(&self) -> [u8; 32] {
        self.segment.segment_hash()
    }

    pub fn bundle_hash(&self) -> [u8; 32] {
        self.segment.bundle_hash()
    }

    pub fn record_count(&self) -> u32 {
        self.segment.record_count()
    }

    pub fn min_hlc(&self) -> Option<HlcTimestamp> {
        self.segment.min_hlc()
    }

    pub fn max_hlc(&self) -> Option<HlcTimestamp> {
        self.segment.max_hlc()
    }

    pub fn into_inner(self) -> Segment {
        self.segment
    }

    pub fn flush(&mut self) -> Result<(), SegmentError> {
        self.segment.flush()
    }

    pub fn seal_and_flush(&mut self) -> Result<[u8; 32], SegmentError> {
        let seg_id = self.segment.seal()?;
        self.segment.flush()?;
        Ok(seg_id)
    }

    pub fn segment(&self) -> &Segment {
        &self.segment
    }
}

pub struct SegmentReader {
    segment: Segment,
}

impl SegmentReader {
    pub fn from_file(path: std::path::PathBuf) -> Result<Self, SegmentError> {
        let segment = Segment::from_file(path)?;
        Ok(Self { segment })
    }

    pub fn from_encrypted_file(
        path: std::path::PathBuf,
        store_key: [u8; 32],
        store_uuid: [u8; 16],
    ) -> Result<Self, SegmentError> {
        let segment = Segment::from_encrypted_file(path, store_key, store_uuid)?;
        Ok(Self { segment })
    }

    pub fn from_segment(segment: Segment) -> Self {
        Self { segment }
    }

    pub fn is_sealed(&self) -> bool {
        self.segment.is_sealed()
    }

    pub fn segment_id(&self) -> [u8; 32] {
        self.segment.segment_id()
    }

    pub fn segment_hash(&self) -> [u8; 32] {
        self.segment.segment_hash()
    }

    pub fn bundle_hash(&self) -> [u8; 32] {
        self.segment.bundle_hash()
    }

    pub fn record_count(&self) -> u32 {
        self.segment.record_count()
    }

    pub fn min_hlc(&self) -> Option<HlcTimestamp> {
        self.segment.min_hlc()
    }

    pub fn max_hlc(&self) -> Option<HlcTimestamp> {
        self.segment.max_hlc()
    }

    pub fn get_event_at(&self, offset: u64) -> Result<Event, SegmentError> {
        self.segment.get_event_at(offset)
    }

    pub fn data_len(&self) -> usize {
        self.segment.data().len()
    }

    pub fn iter_events(&self) -> SegmentEventIterator<'_> {
        self.segment.iter_events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::ActorId;
    use tempfile::TempDir;

    #[test]
    fn test_segment_new() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let segment = Segment::new(path, 1024 * 1024);

        assert!(!segment.is_sealed());
        assert_eq!(segment.record_count(), 0);
    }

    #[test]
    fn test_segment_append_event() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();
        let event = Event::new(stream_id, actor_id, b"test payload".to_vec());

        let offset = segment.append_event(&event)?;
        assert_eq!(offset, 0);
        assert_eq!(segment.record_count(), 1);

        Ok(())
    }

    #[test]
    fn test_segment_seal() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();

        for i in 0..10 {
            let event = Event::new(
                stream_id.clone(),
                actor_id.clone(),
                format!("payload {i}").into_bytes(),
            );
            segment.append_event(&event)?;
        }

        let segment_id = segment.seal()?;

        assert!(segment.is_sealed());
        assert!(!segment_id.iter().all(|&b| b == 0));

        Ok(())
    }

    #[test]
    fn test_segment_seal_twice() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();
        let event = Event::new(stream_id, actor_id, b"test".to_vec());
        segment.append_event(&event)?;

        segment.seal()?;

        let result = segment.seal();
        assert!(matches!(result, Err(SegmentError::AlreadySealed)));

        Ok(())
    }

    #[test]
    fn test_segment_merkle_root() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();

        for i in 0..5 {
            let event = Event::new(
                stream_id.clone(),
                actor_id.clone(),
                format!("payload {i}").into_bytes(),
            );
            segment.append_event(&event)?;
        }

        segment.seal()?;

        let root = segment.merkle_root();
        assert!(!root.iter().all(|&b| b == 0));

        Ok(())
    }

    #[test]
    fn test_segment_hlc_tracking() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();

        let event1 = Event::new(stream_id.clone(), actor_id.clone(), b"test1".to_vec());
        segment.append_event(&event1)?;

        let event2 = Event::new(stream_id, actor_id, b"test2".to_vec());
        segment.append_event(&event2)?;

        segment.seal()?;

        let min = segment.min_hlc().unwrap();
        let max = segment.max_hlc().unwrap();

        assert!(min <= max);

        Ok(())
    }

    #[test]
    fn test_segment_full() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 100);

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();

        let result = segment.append_event(&Event::new(stream_id, actor_id, vec![0u8; 200]));
        assert!(matches!(result, Err(SegmentError::Full)));

        Ok(())
    }

    #[test]
    fn test_segment_writer() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut writer = SegmentWriter::new(path, 1024 * 1024);

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();

        for i in 0..5 {
            let event = Event::new(
                stream_id.clone(),
                actor_id.clone(),
                format!("event {i}").into_bytes(),
            );
            writer.append(&event)?;
        }

        let segment_id = writer.seal()?;

        assert!(!segment_id.iter().all(|&b| b == 0));

        Ok(())
    }

    #[test]
    fn test_get_event_at() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();
        let event = Event::new(stream_id, actor_id.clone(), b"test payload".to_vec());
        let offset = segment.append_event(&event)?;

        let retrieved = segment.get_event_at(offset)?;

        assert_eq!(retrieved.payload, b"test payload");

        Ok(())
    }

    #[test]
    fn test_segment_from_file_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        // Write invalid data
        std::fs::write(&path, b"invalid").unwrap();

        let result = Segment::from_file(path);
        assert!(matches!(result, Err(SegmentError::InvalidSegment)));
    }

    #[test]
    fn test_segment_from_file_empty() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        // Write empty file
        std::fs::write(&path, b"").unwrap();

        let result = Segment::from_file(path);
        assert!(matches!(result, Err(SegmentError::InvalidSegment)));
    }

    #[test]
    fn test_segment_flush_and_read() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();

        // Write and flush
        {
            let mut segment = Segment::new(path.clone(), 1024 * 1024);
            segment.append_event(&Event::new(
                stream_id.clone(),
                actor_id.clone(),
                b"test1".to_vec(),
            ))?;
            segment.append_event(&Event::new(stream_id.clone(), actor_id, b"test2".to_vec()))?;
            segment.flush()?;
        }

        // Read back
        let segment = Segment::from_file(path)?;
        assert_eq!(segment.record_count(), 2);
        assert!(segment.is_sealed());

        Ok(())
    }

    #[test]
    fn test_segment_get_event_out_of_bounds() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let segment = Segment::new(path, 1024 * 1024);

        let result = segment.get_event_at(1000);
        assert!(matches!(result, Err(SegmentError::InvalidSegment)));

        Ok(())
    }

    #[test]
    fn test_segment_sync() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);
        segment.append_event(&Event::new(
            StreamId::new(),
            ActorId::new(),
            b"test".to_vec(),
        ))?;
        segment.flush()?;

        // Now sync
        segment.sync()?;

        Ok(())
    }

    #[test]
    fn test_segment_append_to_sealed() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);
        segment.append_event(&Event::new(
            StreamId::new(),
            ActorId::new(),
            b"test".to_vec(),
        ))?;
        segment.seal()?;

        // Try to append to sealed segment
        let result = segment.append_event(&Event::new(
            StreamId::new(),
            ActorId::new(),
            b"test2".to_vec(),
        ));
        assert!(matches!(result, Err(SegmentError::AlreadySealed)));

        Ok(())
    }

    #[test]
    fn test_segment_header_info() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);
        segment.append_event(&Event::new(
            StreamId::new(),
            ActorId::new(),
            b"test".to_vec(),
        ))?;

        assert_eq!(segment.segment_id(), [0u8; 32]);
        assert_eq!(segment.record_count(), 1);
        assert!(segment.min_hlc().is_some());
        assert!(segment.max_hlc().is_some());

        Ok(())
    }

    #[test]
    fn test_segment_data() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment.bin");

        let mut segment = Segment::new(path, 1024 * 1024);
        segment.append_event(&Event::new(
            StreamId::new(),
            ActorId::new(),
            b"test data".to_vec(),
        ))?;

        let data = segment.data();
        assert!(!data.is_empty());

        Ok(())
    }

    #[test]
    fn test_bundle_hash_separate_from_segment_hash() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path_a = temp_dir.path().join("segment_a.bin");
        let path_b = temp_dir.path().join("segment_b.bin");

        let mut a = Segment::new(path_a, 1024 * 1024);
        let mut b = Segment::new(path_b, 1024 * 1024);
        let hlc = HlcTimestamp {
            physical: 123,
            logical: 0,
        };
        let payload = b"canonical-bundle-payload";
        a.append_serialized(payload, hlc)?;
        b.append_serialized(payload, hlc)?;

        // Keep storage hash dependent on storage metadata (created_hlc).
        b.header.created_hlc = HlcTimestamp {
            physical: 456,
            logical: 0,
        };

        let a_bundle_hash = a.bundle_hash();
        let b_bundle_hash = b.bundle_hash();
        assert_eq!(a_bundle_hash, b_bundle_hash);

        let a_segment_hash = a.seal()?;
        let b_segment_hash = b.seal()?;
        assert_ne!(a_segment_hash, b_segment_hash);
        assert_eq!(a.segment_hash(), a_segment_hash);
        assert_eq!(b.segment_hash(), b_segment_hash);

        Ok(())
    }

    #[test]
    fn test_segment_encrypted_flush_and_read() -> Result<(), SegmentError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment_encrypted.bin");

        let actor_id = ActorId::new();
        let stream_id = StreamId::new();
        let store_uuid = [0x55; 16];
        let store_key = [0xAA; 32];

        {
            let mut writer = SegmentWriter::new(path.clone(), 1024 * 1024);
            writer.enable_encryption(SegmentEncryptionConfig::payload_only(store_uuid, store_key));
            writer.append(&Event::new(
                stream_id.clone(),
                actor_id.clone(),
                b"secret-1".to_vec(),
            ))?;
            writer.append(&Event::new(stream_id, actor_id, b"secret-2".to_vec()))?;
            writer.seal_and_flush()?;
        }

        let reader = SegmentReader::from_encrypted_file(path, store_key, store_uuid)?;
        assert_eq!(reader.record_count(), 2);
        let mut it = reader.iter_events();
        let e1 = it.next().expect("event1")?;
        let e2 = it.next().expect("event2")?;
        assert_eq!(e1.payload, b"secret-1");
        assert_eq!(e2.payload, b"secret-2");
        Ok(())
    }
}
