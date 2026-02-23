// SPDX-License-Identifier: GPL-2.0-only
//! High-performance async segment writer with io_uring integration.
//!
//! This provides async segment writes for improved throughput:
//! - Batched I/O operations
//! - Async fsync (non-blocking)
//! - Parallel flushes

use crate::durability::DurabilityLevel;
use crate::encryption::SegmentEncryptionConfig;
use crate::event::{Event, HlcTimestamp};
use crate::io_reactor::{IoFileHandle, IoReactor, IoReactorConfig, IoTicket};
use crate::segment::{Segment, SegmentError};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MIN_DISK_WRITE: usize = 4096;
const WRITE_CHUNK_TARGET: usize = 1024 * 1024;
const MAX_INFLIGHT_WRITES: usize = 32;
const WRITE_BATCH_MAX_CHUNKS: usize = 16;
const AUTO_FLUSH_TARGET_INFLIGHT: usize = 16;
const AUTO_FLUSH_LOW_WATERMARK: usize = 8;
const AUTO_FLUSH_HIGH_WATERMARK: usize = 24;
const AUTO_FLUSH_LATENCY: Duration = Duration::from_millis(8);
const AUTO_FLUSH_MIN_BYTES: usize = 512 * 1024;

struct PendingWrite {
    ticket: IoTicket<usize>,
    expected_bytes: usize,
}

/// Async segment writer using io_uring.
pub struct AsyncSegmentWriter {
    base_path: std::path::PathBuf,
    max_size: u64,
    segment_index: u64,
    segment: Segment,
    io_reactor: Arc<IoReactor>,
    file: IoFileHandle,
    manifest_file: Option<IoFileHandle>,
    file_offset: u64,
    staged_chunk: Vec<u8>,
    ready_chunks: VecDeque<Vec<u8>>,
    ready_bytes: usize,
    pending_writes: VecDeque<PendingWrite>,
    last_auto_flush: Instant,
    encryption_config: Option<SegmentEncryptionConfig>,
}

impl Drop for AsyncSegmentWriter {
    fn drop(&mut self) {
        if let Some(manifest) = self.manifest_file {
            self.io_reactor.close(manifest);
        }
        self.io_reactor.close(self.file);
    }
}

impl AsyncSegmentWriter {
    /// Create a new async segment writer.
    pub fn new(
        path: std::path::PathBuf,
        max_size: u64,
        io_reactor: Arc<IoReactor>,
    ) -> Result<Self, SegmentError> {
        let file = Self::open_segment_file(&io_reactor, &path, max_size)?;

        Ok(Self {
            base_path: path.clone(),
            max_size,
            segment_index: 0,
            segment: Segment::new(path, max_size),
            io_reactor,
            file,
            manifest_file: None,
            file_offset: 0,
            staged_chunk: Vec::with_capacity(WRITE_CHUNK_TARGET),
            ready_chunks: VecDeque::with_capacity(MAX_INFLIGHT_WRITES * 2),
            ready_bytes: 0,
            pending_writes: VecDeque::with_capacity(MAX_INFLIGHT_WRITES * 2),
            last_auto_flush: Instant::now(),
            encryption_config: None,
        })
    }

    pub fn enable_encryption(&mut self, config: SegmentEncryptionConfig) {
        self.segment.enable_encryption(config.clone());
        self.encryption_config = Some(config);
    }

    /// Append an event to the segment (buffered in memory).
    pub fn append(&mut self, event: &Event) -> Result<u64, SegmentError> {
        let serialized = event.serialize()?;
        self.append_serialized(&serialized, event.hlc_timestamp)
    }

    /// Append an already-encoded event payload to the segment.
    pub fn append_serialized(
        &mut self,
        serialized: &[u8],
        hlc: HlcTimestamp,
    ) -> Result<u64, SegmentError> {
        let offset = match self.segment.append_serialized(serialized, hlc) {
            Ok(offset) => offset,
            Err(SegmentError::Full) => {
                self.roll_segment()?;
                self.segment.append_serialized(serialized, hlc)?
            }
            Err(e) => return Err(e),
        };
        if self.encryption_config.is_none() {
            self.staged_chunk.extend_from_slice(serialized);
            self.promote_ready_chunks();
            self.auto_flush_if_needed(DurabilityLevel::AckBuffered)?;
        }
        Ok(offset)
    }

    /// Flush pending writes using io_uring for async I/O.
    pub fn flush_async(&mut self) -> Result<(), SegmentError> {
        if self.encryption_config.is_some() {
            return self.flush_encrypted(DurabilityLevel::AckBuffered);
        }
        self.flush_chunks(DurabilityLevel::AckBuffered, false)
    }

    /// Flush and wait for completion with fsync.
    pub fn flush_blocking(&mut self) -> Result<(), SegmentError> {
        self.flush_with_durability(DurabilityLevel::AckDurable)
    }

    /// Seal the segment (flush + finalize).
    pub fn seal(&mut self) -> Result<[u8; 32], SegmentError> {
        if self.encryption_config.is_some() {
            // In encrypted mode, sealing requires rebuilding final segment bytes once.
            let seg_id = self.segment.seal()?;
            let final_bytes = self.segment.serialize_result()?;
            self.write_full_segment(final_bytes, DurabilityLevel::AckDurable)?;
            self.staged_chunk.clear();
            self.ready_chunks.clear();
            self.ready_bytes = 0;
            return Ok(seg_id);
        }

        self.flush_chunks(DurabilityLevel::AckDurable, true)?;

        let seg_id = self.segment.seal()?;
        let final_bytes = self.segment.serialize_result()?;

        self.io_reactor.truncate(self.file, 0).wait()?;
        self.io_reactor
            .write_and_fsync(self.file, 0, final_bytes.clone(), false)
            .wait()?;

        self.file_offset = final_bytes.len() as u64;
        self.staged_chunk.clear();
        self.ready_chunks.clear();
        self.ready_bytes = 0;

        Ok(seg_id)
    }

    pub fn is_sealed(&self) -> bool {
        self.segment.is_sealed()
    }

    pub fn segment_id(&self) -> [u8; 32] {
        self.segment.segment_id()
    }

    pub fn record_count(&self) -> u32 {
        self.segment.record_count()
    }

    pub fn attach_manifest(
        &mut self,
        manifest_path: std::path::PathBuf,
    ) -> Result<(), SegmentError> {
        if let Some(handle) = self.manifest_file.take() {
            self.io_reactor.close(handle);
        }
        let handle = self
            .io_reactor
            .open_file_buffered(manifest_path, true, true, true, true)?;
        self.manifest_file = Some(handle);
        Ok(())
    }

    pub fn flush_checkpointed(&mut self, manifest_bytes: Vec<u8>) -> Result<(), SegmentError> {
        if self.encryption_config.is_some() {
            let segment_bytes = self.segment.serialize_result()?;
            self.write_full_segment(segment_bytes, DurabilityLevel::AckDurable)?;
        }
        self.reap_pending(true)?;

        let manifest_handle = self.manifest_file.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "manifest file is not attached",
            )
        })?;

        let mut segment_chunks = self.take_all_pending_chunks();

        self.io_reactor
            .truncate(manifest_handle, manifest_bytes.len() as u64)
            .wait()?;

        if segment_chunks.is_empty() {
            self.io_reactor
                .write_and_fsync(manifest_handle, 0, manifest_bytes, false)
                .wait()?;
        } else {
            let expected: usize = segment_chunks.iter().map(Vec::len).sum();
            let written = if segment_chunks.len() == 1 {
                self.io_reactor
                    .checkpoint_write_fsync(
                        self.file,
                        self.file_offset,
                        segment_chunks.pop().expect("single chunk"),
                        manifest_handle,
                        0,
                        manifest_bytes,
                        false,
                    )
                    .wait()?
            } else {
                self.io_reactor
                    .checkpoint_write_batch_fsync(
                        self.file,
                        self.file_offset,
                        segment_chunks,
                        manifest_handle,
                        0,
                        manifest_bytes,
                        false,
                    )
                    .wait()?
            };
            if written != expected {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    format!("short write completion: expected {expected}, got {written}"),
                )
                .into());
            }
            self.file_offset += written as u64;
        }

        Ok(())
    }

    pub fn flush_with_durability(
        &mut self,
        durability: DurabilityLevel,
    ) -> Result<(), SegmentError> {
        if self.encryption_config.is_some() {
            return self.flush_encrypted(durability);
        }
        match durability {
            DurabilityLevel::AckCheckpointed => {
                self.flush_chunks(DurabilityLevel::AckDurable, true)
            }
            _ => self.flush_chunks(durability, true),
        }
    }

    fn flush_encrypted(&mut self, durability: DurabilityLevel) -> Result<(), SegmentError> {
        self.reap_pending(true)?;
        let segment_bytes = self.segment.serialize_result()?;
        self.write_full_segment(segment_bytes, durability)?;
        self.staged_chunk.clear();
        self.ready_chunks.clear();
        self.ready_bytes = 0;
        Ok(())
    }

    fn write_full_segment(
        &mut self,
        segment_bytes: Vec<u8>,
        durability: DurabilityLevel,
    ) -> Result<(), SegmentError> {
        let len = segment_bytes.len() as u64;
        self.io_reactor.truncate(self.file, 0).wait()?;
        match durability {
            DurabilityLevel::AckDurable
            | DurabilityLevel::AckCheckpointed
            | DurabilityLevel::AckReplicatedN(_) => {
                self.io_reactor
                    .write_and_fsync(self.file, 0, segment_bytes, false)
                    .wait()?;
            }
            DurabilityLevel::AckLocal | DurabilityLevel::AckBuffered => {
                self.io_reactor.write(self.file, 0, segment_bytes).wait()?;
            }
        }
        self.file_offset = len;
        Ok(())
    }

    fn flush_chunks(
        &mut self,
        durability: DurabilityLevel,
        force_all: bool,
    ) -> Result<(), SegmentError> {
        if force_all {
            return self.flush_force_all(durability);
        }

        while self.pending_writes.len() < MAX_INFLIGHT_WRITES {
            while self.pending_writes.len() >= MAX_INFLIGHT_WRITES {
                self.reap_pending(true)?;
            }
            if !self.submit_chunk_batch(WRITE_BATCH_MAX_CHUNKS, false)? {
                break;
            }

            if !force_all && self.pending_writes.len() >= AUTO_FLUSH_HIGH_WATERMARK {
                break;
            }
        }

        Ok(())
    }

    fn flush_force_all(&mut self, durability: DurabilityLevel) -> Result<(), SegmentError> {
        // First drain any in-flight async writes so file_offset and durability semantics stay ordered.
        self.reap_pending(true)?;

        let mut segment_chunks = self.take_all_pending_chunks();
        if segment_chunks.is_empty() {
            if matches!(
                durability,
                DurabilityLevel::AckDurable
                    | DurabilityLevel::AckCheckpointed
                    | DurabilityLevel::AckReplicatedN(_)
            ) {
                self.io_reactor.fsync(self.file, false).wait()?;
            }
            return Ok(());
        }

        let expected: usize = segment_chunks.iter().map(Vec::len).sum();
        let written = match durability {
            DurabilityLevel::AckLocal | DurabilityLevel::AckBuffered => {
                if segment_chunks.len() == 1 {
                    self.io_reactor
                        .write(
                            self.file,
                            self.file_offset,
                            segment_chunks.pop().expect("single chunk"),
                        )
                        .wait()?
                } else {
                    self.io_reactor
                        .write_batch(self.file, self.file_offset, segment_chunks)
                        .wait()?
                }
            }
            DurabilityLevel::AckDurable
            | DurabilityLevel::AckCheckpointed
            | DurabilityLevel::AckReplicatedN(_) => {
                if segment_chunks.len() == 1 {
                    self.io_reactor
                        .write_and_fsync(
                            self.file,
                            self.file_offset,
                            segment_chunks.pop().expect("single chunk"),
                            false,
                        )
                        .wait()?
                } else {
                    self.io_reactor
                        .write_batch_and_fsync(self.file, self.file_offset, segment_chunks, false)
                        .wait()?
                }
            }
        };
        if written != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                format!("short write completion: expected {expected}, got {written}"),
            )
            .into());
        }

        self.file_offset += written as u64;
        self.last_auto_flush = Instant::now();
        Ok(())
    }

    fn take_all_pending_chunks(&mut self) -> Vec<Vec<u8>> {
        self.promote_ready_chunks();

        let mut out = Vec::with_capacity(self.ready_chunks.len() + 1);
        while let Some(chunk) = self.ready_chunks.pop_front() {
            out.push(chunk);
        }
        self.ready_bytes = 0;
        if !self.staged_chunk.is_empty() {
            out.push(std::mem::take(&mut self.staged_chunk));
        }
        out
    }

    fn auto_flush_if_needed(&mut self, _durability: DurabilityLevel) -> Result<(), SegmentError> {
        if self.buffered_bytes() < AUTO_FLUSH_MIN_BYTES
            && self.last_auto_flush.elapsed() < AUTO_FLUSH_LATENCY
        {
            return Ok(());
        }

        if self.pending_writes.len() > AUTO_FLUSH_HIGH_WATERMARK {
            self.reap_pending(false)?;
            return Ok(());
        }

        if self.pending_writes.len() <= AUTO_FLUSH_LOW_WATERMARK
            || self.buffered_bytes() >= WRITE_CHUNK_TARGET
            || (self.last_auto_flush.elapsed() >= AUTO_FLUSH_LATENCY
                && self.buffered_bytes() >= AUTO_FLUSH_MIN_BYTES)
        {
            while self.pending_writes.len() < AUTO_FLUSH_HIGH_WATERMARK {
                if !self.submit_chunk_batch(WRITE_BATCH_MAX_CHUNKS, false)? {
                    break;
                }
            }

            if self.pending_writes.len() > AUTO_FLUSH_HIGH_WATERMARK {
                self.reap_pending(false)?;
            } else if self.pending_writes.len() < AUTO_FLUSH_TARGET_INFLIGHT
                && self.buffered_bytes() >= WRITE_CHUNK_TARGET
            {
                // Keep filling if we still have enough buffered bytes and are below target.
                while self.pending_writes.len() < AUTO_FLUSH_TARGET_INFLIGHT {
                    if !self.submit_chunk_batch(WRITE_BATCH_MAX_CHUNKS, false)? {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn submit_chunk_batch(
        &mut self,
        max_chunks: usize,
        force_all: bool,
    ) -> Result<bool, SegmentError> {
        let mut chunks: Vec<Vec<u8>> = Vec::with_capacity(max_chunks.max(1));
        let mut expected = 0usize;

        while chunks.len() < max_chunks.max(1) {
            let Some(chunk) = self.take_next_chunk(force_all) else {
                break;
            };
            expected = expected.saturating_add(chunk.len());
            chunks.push(chunk);
        }

        if chunks.is_empty() {
            return Ok(false);
        }

        let ticket = if chunks.len() == 1 {
            self.io_reactor.write(
                self.file,
                self.file_offset,
                chunks.pop().expect("single chunk"),
            )
        } else {
            self.io_reactor
                .write_batch(self.file, self.file_offset, chunks)
        };

        self.pending_writes.push_back(PendingWrite {
            ticket,
            expected_bytes: expected,
        });
        self.file_offset += expected as u64;
        self.last_auto_flush = Instant::now();
        Ok(true)
    }

    fn reap_pending(&mut self, drain_all: bool) -> Result<(), SegmentError> {
        if drain_all {
            while let Some(pending) = self.pending_writes.pop_front() {
                let written = pending.ticket.wait()?;
                if written != pending.expected_bytes {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        format!(
                            "short write completion: expected {}, got {}",
                            pending.expected_bytes, written
                        ),
                    )
                    .into());
                }
            }
            return Ok(());
        }

        loop {
            let Some(pending) = self.pending_writes.pop_front() else {
                return Ok(());
            };

            match pending.ticket.try_wait() {
                Some(result) => {
                    let written = result?;
                    if written != pending.expected_bytes {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::WriteZero,
                            format!(
                                "short write completion: expected {}, got {}",
                                pending.expected_bytes, written
                            ),
                        )
                        .into());
                    }
                }
                None => {
                    self.pending_writes.push_front(pending);
                    return Ok(());
                }
            }
        }
    }

    fn take_next_chunk(&mut self, force_all: bool) -> Option<Vec<u8>> {
        self.promote_ready_chunks();

        if let Some(chunk) = self.ready_chunks.pop_front() {
            self.ready_bytes = self.ready_bytes.saturating_sub(chunk.len());
            return Some(chunk);
        }

        if self.staged_chunk.is_empty() {
            return None;
        }

        if !force_all {
            let aligned = self.staged_chunk.len() - (self.staged_chunk.len() % MIN_DISK_WRITE);
            if aligned < MIN_DISK_WRITE {
                return None;
            }
            let tail = self.staged_chunk.split_off(aligned);
            return Some(std::mem::replace(&mut self.staged_chunk, tail));
        }

        Some(std::mem::take(&mut self.staged_chunk))
    }

    fn promote_ready_chunks(&mut self) {
        while self.staged_chunk.len() >= WRITE_CHUNK_TARGET {
            let tail = self.staged_chunk.split_off(WRITE_CHUNK_TARGET);
            let chunk = std::mem::replace(&mut self.staged_chunk, tail);
            self.ready_bytes += chunk.len();
            self.ready_chunks.push_back(chunk);
        }
    }

    fn buffered_bytes(&self) -> usize {
        self.ready_bytes + self.staged_chunk.len()
    }

    fn open_segment_file(
        io_reactor: &Arc<IoReactor>,
        path: &std::path::Path,
        max_size: u64,
    ) -> Result<IoFileHandle, SegmentError> {
        let file = io_reactor.open_file(path, true, true, true, true)?;
        if let Err(e) = io_reactor.preallocate(file, max_size).wait() {
            let raw = e.raw_os_error();
            if raw != Some(libc::EOPNOTSUPP)
                && raw != Some(libc::ENOSYS)
                && raw != Some(libc::EINVAL)
            {
                return Err(e.into());
            }
        }
        Ok(file)
    }

    fn rolled_path(&self, next_index: u64) -> std::path::PathBuf {
        if next_index == 0 {
            return self.base_path.clone();
        }

        let parent = self
            .base_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let stem = self
            .base_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("segment");
        let ext = self.base_path.extension().and_then(|e| e.to_str());

        let file_name = match ext {
            Some(ext) if !ext.is_empty() => format!("{stem}.{next_index}.{ext}"),
            _ => format!("{stem}.{next_index}"),
        };
        parent.join(file_name)
    }

    fn roll_segment(&mut self) -> Result<(), SegmentError> {
        self.flush_chunks(DurabilityLevel::AckDurable, true)?;
        let _ = self.segment.seal()?;
        self.io_reactor.close(self.file);

        self.segment_index = self.segment_index.saturating_add(1);
        let next_path = self.rolled_path(self.segment_index);
        self.file = Self::open_segment_file(&self.io_reactor, &next_path, self.max_size)?;
        self.segment = Segment::new(next_path, self.max_size);
        if let Some(cfg) = &self.encryption_config {
            self.segment.enable_encryption(cfg.clone());
        }
        self.file_offset = 0;
        self.staged_chunk.clear();
        self.ready_chunks.clear();
        self.ready_bytes = 0;
        self.pending_writes.clear();
        self.last_auto_flush = Instant::now();

        Ok(())
    }
}

/// Factory for creating async segment writers with shared io_uring backend.
pub struct AsyncSegmentWriterFactory {
    io_reactor: Arc<IoReactor>,
}

impl AsyncSegmentWriterFactory {
    pub fn new() -> std::io::Result<Self> {
        Self::new_with_options(false, false)
    }

    pub fn new_direct_io() -> std::io::Result<Self> {
        Self::new_with_options(true, true)
    }

    fn new_with_options(use_o_direct: bool, use_o_dsync: bool) -> std::io::Result<Self> {
        let io_reactor = IoReactor::global_with_config(IoReactorConfig {
            queue_depth: 1024,
            batch_size: 128,
            max_batch_latency: Duration::from_micros(50),
            use_sqpoll: false,
            sqpoll_idle_ms: 2000,
            registered_files: 2048,
            fixed_buffer_count: 256,
            fixed_buffer_size: WRITE_CHUNK_TARGET,
            use_o_direct,
            use_o_dsync,
        })?;
        Ok(Self { io_reactor })
    }

    pub fn create_writer(
        &self,
        path: std::path::PathBuf,
        max_size: u64,
    ) -> Result<AsyncSegmentWriter, SegmentError> {
        AsyncSegmentWriter::new(path, max_size, Arc::clone(&self.io_reactor))
    }

    pub fn stats(&self) -> crate::io_reactor::IoReactorStats {
        self.io_reactor.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_async_segment_writer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.seg");

        let factory = AsyncSegmentWriterFactory::new();
        if let Ok(factory) = factory {
            let writer = factory.create_writer(path, 1024 * 1024);
            assert!(writer.is_ok());
        }
    }

    #[test]
    fn test_async_segment_writer_rolls_on_full() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("roll.seg");

        let factory = AsyncSegmentWriterFactory::new();
        if let Ok(factory) = factory {
            let mut writer = factory.create_writer(path.clone(), 4096).unwrap();

            let stream_id = crate::event::StreamId::new();
            let actor_id = crate::event::ActorId::new();
            let payload = vec![0xAA; 2048];
            let event = crate::event::Event::new(stream_id, actor_id, payload);
            let serialized = event.serialize().unwrap();

            let first = writer
                .append_serialized(&serialized, event.hlc_timestamp)
                .unwrap();
            let second = writer
                .append_serialized(&serialized, event.hlc_timestamp)
                .unwrap();

            assert_eq!(first, 0);
            assert_eq!(second, 0);
            assert!(path.exists());
            assert!(temp_dir.path().join("roll.1.seg").exists());
        }
    }
}
