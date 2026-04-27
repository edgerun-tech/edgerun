//! Block-device-backed event log implementation.
//!
//! This backend stores the same varint-framed protobuf event records used by the
//! filesystem backend, but on a raw sector-addressable block storage device.

use std::cell::RefCell;
use std::cmp::min;
use std::sync::{Arc, Mutex};

use edgerun_core::protocol::EventEnvelope;
use edgerun_proto::edgerun::v0::stream::EventEnvelope as ProtoEventEnvelope;
use prost::Message;

use crate::core::{
    AppendReceipt, EventLocation, EventLog, ScannedEvent, canonical_event_hash, encode_event_frame,
};
use crate::error::StorageError;

const LOG_MAGIC: &[u8; 4] = b"ERLG";
const LOG_HEADER_SIZE: usize = 16;
const HEADER_SECTOR: u64 = 0;
const DATA_START_SECTOR: u64 = 1;
const MAX_VARINT_BYTES: u32 = 10;

/// Trait matching the raw block storage contract used by `edgerun-bare-rt`.
///
/// Implementations can be backed by FAT/ATA/NVMe in unikernel, or by a small
/// in-memory or file-based test adapter in hosted builds.
pub trait BlockStorage: Send {
    fn sector_size(&self) -> usize;
    fn sectors(&self) -> u64;
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), StorageError>;
    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), StorageError>;
    fn sync(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

impl BlockStorage for Box<dyn BlockStorage + Send> {
    fn sector_size(&self) -> usize {
        self.as_ref().sector_size()
    }

    fn sectors(&self) -> u64 {
        self.as_ref().sectors()
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        self.as_mut().read_sector(sector, buf)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), StorageError> {
        self.as_mut().write_sector(sector, buf)
    }

    fn sync(&mut self) -> Result<(), StorageError> {
        self.as_mut().sync()
    }
}

/// Block-backed append-only event log.
pub struct BlockEventLog<S: BlockStorage> {
    device: RefCell<S>,
    sector_size: usize,
    sectors: u64,
    cursor: u64,
}

impl<S: BlockStorage> BlockEventLog<S> {
    pub fn open(device: S) -> Result<Self, StorageError> {
        let mut log = Self {
            sector_size: device.sector_size(),
            sectors: device.sectors(),
            cursor: 0,
            device: RefCell::new(device),
        };

        log.init_or_recover()?;
        Ok(log)
    }

    fn data_region_bytes(&self) -> u64 {
        self.sectors
            .saturating_sub(DATA_START_SECTOR)
            .saturating_mul(self.sector_size as u64)
    }

    fn data_start_byte(&self) -> u64 {
        self.sector_size as u64 * DATA_START_SECTOR
    }

    fn parse_header(header: &[u8]) -> Option<u64> {
        if header.len() < LOG_HEADER_SIZE {
            return None;
        }
        if &header[..4] != LOG_MAGIC {
            return None;
        }
        let cursor = u64::from_le_bytes(header[4..12].try_into().ok()?);
        Some(cursor)
    }

    fn write_header(&self) -> Result<(), StorageError> {
        let mut buf = vec![0u8; self.sector_size];
        buf[..4].copy_from_slice(LOG_MAGIC);
        buf[4..12].copy_from_slice(&self.cursor.to_le_bytes());
        buf[12] = 1;
        {
            let mut device = self.device.borrow_mut();
            device.write_sector(HEADER_SECTOR, &buf)?;
            device.sync()?;
        }
        Ok(())
    }

    fn init_or_recover(&mut self) -> Result<(), StorageError> {
        if self.sector_size < LOG_HEADER_SIZE {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "sector size smaller than event log header",
            )));
        }
        if self.sectors == 0 {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "block device has zero sectors",
            )));
        }

        let cursor = match {
            let mut device = self.device.borrow_mut();
            let mut buf = vec![0u8; self.sector_size];
            device.read_sector(HEADER_SECTOR, &mut buf)?;
            Self::parse_header(&buf)
        } {
            Some(v) => v,
            None => 0,
        };

        if cursor > self.data_region_bytes() {
            return Err(StorageError::Decode(
                "event log header cursor exceeds storage".into(),
            ));
        }
        self.cursor = cursor;
        self.write_header()?;
        Ok(())
    }

    fn to_abs_sector_offset(&self, data_offset: u64) -> u64 {
        self.data_start_byte().saturating_add(data_offset)
    }

    fn read_bytes(&self, data_offset: u64, len: usize) -> Result<Vec<u8>, StorageError> {
        let mut remaining = len;
        let mut pos = self.to_abs_sector_offset(data_offset);
        let mut out = vec![0u8; len];
        let mut written = 0usize;

        if data_offset.checked_add(len as u64).is_none()
            || data_offset.saturating_add(len as u64) > self.data_region_bytes()
        {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read crosses device data region",
            )));
        }

        let mut device = self.device.borrow_mut();
        while remaining > 0 {
            let sector = pos / self.sector_size as u64;
            let sector_offset = (pos % self.sector_size as u64) as usize;
            let in_sector = self.sector_size.saturating_sub(sector_offset);
            let copy_len = min(remaining, in_sector);

            let mut sector_data = vec![0u8; self.sector_size];
            device.read_sector(sector, &mut sector_data)?;

            out[written..written + copy_len]
                .copy_from_slice(&sector_data[sector_offset..sector_offset + copy_len]);

            remaining -= copy_len;
            written += copy_len;
            pos = pos.saturating_add(copy_len as u64);
        }

        Ok(out)
    }

    fn write_bytes(&self, data_offset: u64, bytes: &[u8]) -> Result<(), StorageError> {
        let mut remaining = bytes.len();
        let mut pos = self.to_abs_sector_offset(data_offset);
        let mut copied = 0usize;

        if data_offset.checked_add(bytes.len() as u64).is_none()
            || data_offset.saturating_add(bytes.len() as u64) > self.data_region_bytes()
        {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "append would exceed event log capacity",
            )));
        }

        let mut device = self.device.borrow_mut();
        while remaining > 0 {
            let sector = pos / self.sector_size as u64;
            let sector_offset = (pos % self.sector_size as u64) as usize;
            let in_sector = self.sector_size.saturating_sub(sector_offset);
            let copy_len = min(remaining, in_sector);

            let mut sector_data = vec![0u8; self.sector_size];
            device.read_sector(sector, &mut sector_data)?;

            sector_data[sector_offset..sector_offset + copy_len]
                .copy_from_slice(&bytes[copied..copied + copy_len]);

            device.write_sector(sector, &sector_data)?;

            remaining -= copy_len;
            copied += copy_len;
            pos = pos.saturating_add(copy_len as u64);
        }
        device.sync()?;

        Ok(())
    }

    fn append_raw_frame(&mut self, frame: &[u8]) -> Result<u64, StorageError> {
        if self
            .cursor
            .checked_add(frame.len() as u64)
            .is_none_or(|end| end > self.data_region_bytes())
        {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "block event log is full",
            )));
        }

        let offset = self.cursor;
        self.write_bytes(offset, frame)?;
        self.cursor = self.cursor.saturating_add(frame.len() as u64);
        self.write_header()?;
        Ok(offset)
    }

    fn decode_varint_at(&self, data_offset: u64) -> Result<(u64, u8), StorageError> {
        let mut shift: u32 = 0;
        let mut value: u64 = 0;
        let mut consumed: u8 = 0;

        loop {
            if consumed >= MAX_VARINT_BYTES as u8 {
                return Err(StorageError::Decode("event-frame varint too long".into()));
            }

            let byte = self.read_bytes(data_offset.saturating_add(consumed as u64), 1)?[0];
            value |= ((byte & 0x7F) as u64) << shift;
            consumed = consumed.saturating_add(1);

            if byte & 0x80 == 0 {
                return Ok((value, consumed));
            }

            shift = shift.saturating_add(7);
        }
    }

    fn read_frame_at(&self, data_offset: u64) -> Result<Vec<u8>, StorageError> {
        let (len, consumed) = self.decode_varint_at(data_offset)?;
        let start = data_offset.saturating_add(consumed as u64);
        let frame_bytes = self.read_bytes(start, len as usize)?;
        Ok(frame_bytes)
    }
}

impl<S: BlockStorage> EventLog for BlockEventLog<S> {
    fn append_event(&mut self, event: &EventEnvelope) -> Result<AppendReceipt, StorageError> {
        let (len_prefix, event_bytes) = encode_event_frame(event)?;
        let mut frame = Vec::with_capacity(len_prefix.len() + event_bytes.len());
        frame.extend_from_slice(&len_prefix);
        frame.extend_from_slice(&event_bytes);

        let event_hash = canonical_event_hash(event).value;
        let file_offset = self.append_raw_frame(&frame)?;

        Ok(AppendReceipt {
            stream_id: event.stream_id.clone(),
            seq: event.seq,
            event_hash,
            file_offset,
            envelope_version: event.envelope_version,
        })
    }

    fn read_event(
        &self,
        stream_id: &[u8],
        seq: u64,
        location: &EventLocation,
    ) -> Result<Option<EventEnvelope>, StorageError> {
        if location.file_offset >= self.cursor {
            return Ok(None);
        }

        let event_bytes = self.read_frame_at(location.file_offset)?;
        let event = ProtoEventEnvelope::decode(event_bytes.as_slice())
            .map_err(|e| StorageError::Decode(format!("event protobuf decode failed: {e}")))?;

        if event.stream_id != stream_id {
            return Err(StorageError::Decode(format!(
                "event stream mismatch at offset {}: expected {}, got {}",
                location.file_offset,
                edgerun_core::util::bytes_to_hex(stream_id),
                edgerun_core::util::bytes_to_hex(&event.stream_id),
            )));
        }
        if event.seq != seq {
            return Err(StorageError::Decode(format!(
                "event seq mismatch at offset {}: expected {seq}, got {}",
                location.file_offset, event.seq
            )));
        }

        Ok(Some(event))
    }

    fn scan(&self) -> Result<Vec<ScannedEvent>, StorageError> {
        let mut scanned = Vec::new();
        let mut cursor = 0u64;

        while cursor < self.cursor {
            let offset = cursor;
            let (len, consumed) = self.decode_varint_at(cursor)?;
            let data_offset = cursor.saturating_add(consumed as u64);
            let event_bytes = self.read_bytes(data_offset, len as usize)?;
            let event = ProtoEventEnvelope::decode(event_bytes.as_slice())
                .map_err(|e| StorageError::Decode(format!("event protobuf decode failed: {e}")))?;
            let event_hash = canonical_event_hash(&event).value;
            scanned.push(ScannedEvent {
                location: EventLocation {
                    stream_id: event.stream_id.clone(),
                    seq: event.seq,
                    event_hash,
                    file_offset: offset,
                    envelope_version: event.envelope_version,
                },
                event,
            });
            cursor = data_offset.saturating_add(len);
        }

        Ok(scanned)
    }
}

/// Simple in-memory block storage for hosted and test scenarios.
#[derive(Clone)]
pub struct InMemoryBlockDevice {
    sector_size: usize,
    sectors: u64,
    data: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl InMemoryBlockDevice {
    pub fn new(sector_size: usize, sectors: u64) -> Self {
        let mut data = Vec::new();
        data.resize_with(sectors as usize, || vec![0u8; sector_size]);
        Self {
            sector_size,
            sectors,
            data: Arc::new(Mutex::new(data)),
        }
    }
}

impl BlockStorage for InMemoryBlockDevice {
    fn sector_size(&self) -> usize {
        self.sector_size
    }

    fn sectors(&self) -> u64 {
        self.sectors
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        let data = self
            .data
            .lock()
            .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;
        if sector >= self.sectors {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "sector index out of range",
            )));
        }
        if buf.len() != self.sector_size {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "sector buffer size mismatch",
            )));
        }
        buf.copy_from_slice(&data[sector as usize]);
        Ok(())
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), StorageError> {
        let mut data = self
            .data
            .lock()
            .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;
        if sector >= self.sectors {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "sector index out of range",
            )));
        }
        if buf.len() != self.sector_size {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "sector buffer size mismatch",
            )));
        }
        data[sector as usize].copy_from_slice(buf);
        Ok(())
    }
}

#[cfg(feature = "bare-rt")]
impl<T: edgerun_bare_rt::storage::BlockDevice + Send> BlockStorage for T {
    fn sector_size(&self) -> usize {
        edgerun_bare_rt::storage::SECTOR_SIZE
    }

    fn sectors(&self) -> u64 {
        self.sectors()
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        if buf.len() != self.sector_size() {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "sector buffer size mismatch",
            )));
        }

        if self.read_sector(sector, buf) {
            return Ok(());
        }

        Err(StorageError::Io(std::io::Error::other(format!(
            "failed to read sector {} from bare-rt block device",
            sector
        ))))
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), StorageError> {
        if buf.len() != self.sector_size() {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "sector buffer size mismatch",
            )));
        }

        if self.write_sector(sector, buf) {
            return Ok(());
        }

        Err(StorageError::Io(std::io::Error::other(format!(
            "failed to write sector {} to bare-rt block device",
            sector
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_core::protocol::EventEnvelope;

    fn envelope(stream_id: &[u8], seq: u64, event_type: i32) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            event_version: 1,
            stream_id: stream_id.to_vec(),
            seq,
            prev_event_hash: None,
            event_type,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
            ..Default::default()
        }
    }

    #[test]
    fn block_event_log_append_read_scan() {
        let device = InMemoryBlockDevice::new(16, 32);
        let mut log = BlockEventLog::open(device).unwrap();

        let e1 = envelope(b"stream-a", 0, 1);
        let e2 = envelope(b"stream-a", 1, 1);
        let e3 = envelope(b"stream-b", 0, 1);

        let r1 = log.append_event(&e1).unwrap();
        let r2 = log.append_event(&e2).unwrap();
        let r3 = log.append_event(&e3).unwrap();

        let location = EventLocation {
            stream_id: e1.stream_id.clone(),
            seq: 0,
            event_hash: r1.event_hash,
            file_offset: r1.file_offset,
            envelope_version: e1.envelope_version,
        };
        let read = log
            .read_event(&e1.stream_id, 0, &location)
            .unwrap()
            .unwrap();
        assert_eq!(read.seq, 0);

        let all = log.scan().unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(
            all.iter()
                .map(|e| e.location.file_offset)
                .collect::<Vec<_>>(),
            vec![r1.file_offset, r2.file_offset, r3.file_offset]
        );
    }

    #[test]
    fn block_event_log_recovers_cursor() {
        let mut device = InMemoryBlockDevice::new(16, 32);
        let mut log = BlockEventLog::open(device.clone()).unwrap();
        let e1 = envelope(b"stream", 0, 1);
        let e2 = envelope(b"stream", 1, 1);
        let r1 = log.append_event(&e1).unwrap();
        let r2 = log.append_event(&e2).unwrap();
        assert_eq!(r1.file_offset, 0);

        // Re-open against same underlying storage should preserve cursor and append sequence.
        let mut reopened = BlockEventLog::open(device).unwrap();
        let scanned = reopened.scan().unwrap();
        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0].event.seq, 0);
        assert_eq!(scanned[1].event.seq, 1);

        let r3 = reopened.append_event(&e1).unwrap();
        assert!(r3.file_offset > r2.file_offset);
    }

    #[test]
    fn block_event_log_crosses_sector_boundaries() {
        let device = InMemoryBlockDevice::new(16, 16);
        let mut log = BlockEventLog::open(device).unwrap();

        let mut envelope = envelope(b"boundary-stream", 0, 2);
        envelope.stream_id = vec![b'x'; 200];
        let receipt = log.append_event(&envelope).unwrap();
        let location = EventLocation {
            stream_id: envelope.stream_id.clone(),
            seq: 0,
            event_hash: receipt.event_hash,
            file_offset: receipt.file_offset,
            envelope_version: envelope.envelope_version,
        };
        let read = log
            .read_event(&envelope.stream_id, 0, &location)
            .unwrap()
            .unwrap();
        assert_eq!(read.stream_id, envelope.stream_id);
    }
}
