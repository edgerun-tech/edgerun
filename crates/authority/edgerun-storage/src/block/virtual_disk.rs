//! Adapter from edgerun-virtual-disk block backends into storage block devices.

use crate::prelude::v1::*;

use edgerun_virtual_disk::{BlockBackend, BlockDeviceInfo, BlockError};

use crate::block::BlockStorage;
use crate::error::StorageError;

/// Sector-addressable storage backed by an `edgerun-virtual-disk` block backend.
///
/// The adapter does not add authority. It only lets storage use a virtual disk
/// backend as the sector substrate for append-only event logs and derived stores.
#[derive(Debug)]
pub struct VirtualDiskBlockStorage<B> {
    backend: B,
    info: BlockDeviceInfo,
}

impl<B: BlockBackend> VirtualDiskBlockStorage<B> {
    pub fn new(backend: B) -> Result<Self, StorageError> {
        let info = backend.info();
        validate_info(&info)?;
        Ok(Self { backend, info })
    }

    #[must_use]
    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn into_inner(self) -> B {
        self.backend
    }
}

impl<B: BlockBackend> BlockStorage for VirtualDiskBlockStorage<B> {
    fn sector_size(&self) -> usize {
        self.info.block_size as usize
    }

    fn sectors(&self) -> u64 {
        self.info.block_count
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        validate_sector_io(&self.info, sector, buf.len())?;
        self.backend
            .read_blocks(sector, 1, buf)
            .map_err(map_block_error)
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), StorageError> {
        validate_sector_io(&self.info, sector, buf.len())?;
        self.backend
            .write_blocks(sector, 1, buf)
            .map_err(map_block_error)
    }

    fn sync(&mut self) -> Result<(), StorageError> {
        self.backend.flush().map_err(map_block_error)
    }
}

fn validate_info(info: &BlockDeviceInfo) -> Result<(), StorageError> {
    if info.block_size == 0 {
        return Err(StorageError::InvalidArgument(
            "virtual disk block size must be greater than zero".into(),
        ));
    }
    if info.block_count == 0 {
        return Err(StorageError::InvalidArgument(
            "virtual disk block count must be greater than zero".into(),
        ));
    }
    Ok(())
}

fn validate_sector_io(info: &BlockDeviceInfo, sector: u64, len: usize) -> Result<(), StorageError> {
    if sector >= info.block_count {
        return Err(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "sector index out of range",
        )));
    }
    if len != info.block_size as usize {
        return Err(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "sector buffer size mismatch",
        )));
    }
    Ok(())
}

fn map_block_error(error: BlockError) -> StorageError {
    match error {
        BlockError::OutOfRange => StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "virtual disk block request out of range",
        )),
        BlockError::ReadOnly => StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            "virtual disk backend is read-only",
        )),
        BlockError::Misaligned => StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "virtual disk block request is misaligned",
        )),
        BlockError::Unsupported => StorageError::InvalidArgument(
            "virtual disk backend does not support this storage operation".into(),
        ),
        BlockError::BackendFailure(message) => StorageError::Io(std::io::Error::other(message)),
        BlockError::ProtocolError(message) => StorageError::Decode(message),
        BlockError::NotReady => StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "virtual disk backend is not ready",
        )),
        BlockError::Timeout => StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "virtual disk backend timed out",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockEventLog;
    use crate::core::EventLog;
    use edgerun_protocols::core_protocol::protocol::EventEnvelope;
    use edgerun_virtual_disk::{BlockDeviceInfo, MemoryBlockBackend};

    fn info() -> BlockDeviceInfo {
        BlockDeviceInfo {
            block_size: 512,
            block_count: 16,
            readonly: false,
            supports_flush: true,
            supports_discard: false,
            supports_write_zeroes: true,
            model: "storage-test".into(),
            serial: "storage-test-0".into(),
        }
    }

    fn event(stream_id: &[u8], seq: u64) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            event_version: 1,
            stream_id: stream_id.to_vec(),
            seq,
            prev_event_hash: None,
            event_type: 1,
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
    fn virtual_disk_backend_can_back_event_log() {
        let backend = MemoryBlockBackend::new(info()).unwrap();
        let storage = VirtualDiskBlockStorage::new(backend).unwrap();
        let mut log = BlockEventLog::open(storage).unwrap();

        let first = event(b"vdisk-stream", 0);
        let second = event(b"vdisk-stream", 1);
        log.append_event(&first).unwrap();
        log.append_event(&second).unwrap();

        let scanned = log.scan().unwrap();
        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0].event.stream_id, b"vdisk-stream");
        assert_eq!(scanned[1].event.seq, 1);
    }

    #[test]
    fn virtual_disk_storage_rejects_wrong_sector_buffer_size() {
        let backend = MemoryBlockBackend::new(info()).unwrap();
        let mut storage = VirtualDiskBlockStorage::new(backend).unwrap();
        let err = storage.read_sector(0, &mut [0_u8; 128]).unwrap_err();
        assert!(matches!(err, StorageError::Io(_)));
    }
}
