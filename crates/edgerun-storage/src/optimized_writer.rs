// SPDX-License-Identifier: Apache-2.0
use crate::arena::ObjectPool;
use crate::event::{ActorId, Event, StreamId};
use crate::io_reactor::{IoFileHandle, IoReactor};
use crate::segment::SegmentError;
use crate::sharding::{ShardedMap, ShardedWriterPool, ShardingConfig};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Debug)]
pub struct OptimizedSegmentWriterConfig {
    pub num_cores: usize,
    pub buffer_size: usize,
    pub use_io_uring: bool,
    pub batch_size: usize,
}

impl Default for OptimizedSegmentWriterConfig {
    fn default() -> Self {
        Self {
            num_cores: num_cpus::get(),
            buffer_size: DEFAULT_BUFFER_SIZE,
            use_io_uring: true,
            batch_size: 1024,
        }
    }
}

pub struct OptimizedSegmentWriter {
    config: OptimizedSegmentWriterConfig,
    #[allow(dead_code)]
    writer_pool: Arc<ShardedWriterPool>,
    #[allow(dead_code)]
    event_pool: Arc<ObjectPool<SerializedEvent>>,
    segment_map: Arc<ShardedMap<u64, SegmentInfo>>,
    current_segment: Mutex<Option<ActiveSegment>>,
    sealed_segments: Vec<[u8; 32]>,
    events_written: AtomicU64,
    bytes_written: AtomicU64,
    io_reactor: Option<Arc<IoReactor>>,
    #[allow(dead_code)]
    current_file: Mutex<Option<IoFileHandle>>,
}

#[allow(dead_code)]
#[derive(Clone)]
struct SegmentInfo {
    segment_id: [u8; 32],
    record_count: u32,
    size_bytes: u64,
}

struct ActiveSegment {
    path: PathBuf,
    segment_id: [u8; 32],
    buffer: Vec<u8>,
    record_count: u32,
    current_offset: u64,
    min_hlc: Option<crate::event::HlcTimestamp>,
    max_hlc: Option<crate::event::HlcTimestamp>,
    record_offsets: Vec<u64>,
}

impl OptimizedSegmentWriter {
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new(path: PathBuf, config: OptimizedSegmentWriterConfig) -> Self {
        let writer_pool = Arc::new(ShardedWriterPool::new(config.num_cores, config.buffer_size));

        let event_pool = Arc::new(ObjectPool::new(config.num_cores * config.batch_size));

        let sharding_config = ShardingConfig {
            shard_count: config.num_cores * 2,
            numa_aware: false,
            preallocate: true,
        };
        let segment_map = Arc::new(ShardedMap::new(sharding_config));

        let io_reactor = if config.use_io_uring {
            IoReactor::global().ok()
        } else {
            None
        };

        Self {
            config,
            writer_pool,
            event_pool,
            segment_map,
            current_segment: Mutex::new(Some(ActiveSegment::new(path))),
            sealed_segments: Vec::new(),
            events_written: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
            io_reactor,
            current_file: Mutex::new(None),
        }
    }

    pub fn append(
        &mut self,
        stream_id: &StreamId,
        actor_id: &ActorId,
        payload: Vec<u8>,
    ) -> Result<u64, SegmentError> {
        let event = Event::new(stream_id.clone(), actor_id.clone(), payload);
        let serialized = event.serialize()?;
        let serialized_len = serialized.len();

        // Update stats
        self.events_written.fetch_add(1, Ordering::Relaxed);
        self.bytes_written
            .fetch_add(serialized_len as u64, Ordering::Relaxed);

        // Get current offset and update segment
        let mut segment = self.current_segment.lock().unwrap();
        if let Some(ref mut seg) = *segment {
            let offset = seg.current_offset;
            seg.current_offset += serialized_len as u64;
            seg.record_count += 1;
            seg.record_offsets.push(offset);
            seg.buffer.extend_from_slice(&serialized);

            let hlc = event.hlc_timestamp;
            seg.min_hlc = Some(seg.min_hlc.map_or(hlc, |min| min.min(hlc)));
            seg.max_hlc = Some(seg.max_hlc.map_or(hlc, |max| max.max(hlc)));

            if seg.buffer.len() >= self.config.buffer_size {
                drop(segment);
                self.seal_current_segment()?;
            }

            Ok(offset)
        } else {
            Err(SegmentError::AlreadySealed)
        }
    }

    pub fn seal_current_segment(&mut self) -> Result<[u8; 32], SegmentError> {
        let mut segment = self.current_segment.lock().unwrap();
        if let Some(mut seg) = segment.take() {
            let segment_hash = Self::compute_hash(&seg.buffer);
            seg.segment_id = segment_hash;

            let info = SegmentInfo {
                segment_id: segment_hash,
                record_count: seg.record_count,
                size_bytes: seg.buffer.len() as u64,
            };
            self.segment_map.insert(seg.current_offset, info);

            // Write to disk
            if let Some(ref reactor) = self.io_reactor {
                let file = reactor.open_file(&seg.path, true, true, true, true)?;
                reactor.truncate(file, 0).wait()?;
                reactor
                    .write_and_fsync(file, 0, seg.buffer.clone(), false)
                    .wait()?;
                reactor.close(file);
            } else if self.config.use_io_uring {
                return Err(std::io::Error::other(
                    "io_uring is enabled but IoReactor is unavailable",
                )
                .into());
            } else {
                std::fs::write(&seg.path, &seg.buffer)?;
            }

            self.sealed_segments.push(segment_hash);
            Ok(segment_hash)
        } else {
            Err(SegmentError::AlreadySealed)
        }
    }

    pub fn seal(&mut self) -> Result<[u8; 32], SegmentError> {
        self.seal_current_segment()
    }

    pub fn flush(&mut self) -> Result<(), SegmentError> {
        if let Some(ref reactor) = self.io_reactor {
            let _ = reactor.stats();
        }
        Ok(())
    }

    pub fn events_written(&self) -> u64 {
        self.events_written.load(Ordering::Relaxed)
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }

    pub fn sealed_count(&self) -> usize {
        self.sealed_segments.len()
    }

    fn compute_hash(data: &[u8]) -> [u8; 32] {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        *hasher.finalize().as_bytes()
    }
}

impl ActiveSegment {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            segment_id: [0u8; 32],
            buffer: Vec::with_capacity(DEFAULT_BUFFER_SIZE),
            record_count: 0,
            current_offset: 0,
            min_hlc: None,
            max_hlc: None,
            record_offsets: Vec::new(),
        }
    }
}

#[allow(dead_code)]
struct SerializedEvent {
    data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_optimized_writer_append() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment");

        let config = OptimizedSegmentWriterConfig {
            num_cores: 4,
            buffer_size: 1024 * 1024,
            use_io_uring: false,
            batch_size: 100,
        };

        let mut writer = OptimizedSegmentWriter::new(path, config);

        let stream_id = StreamId::new();
        let actor_id = ActorId::new();

        let offset = writer
            .append(&stream_id, &actor_id, b"test payload".to_vec())
            .unwrap();
        assert_eq!(offset, 0);

        let segment_id = writer.seal().unwrap();
        assert!(!segment_id.iter().all(|&b| b == 0));

        assert_eq!(writer.events_written(), 1);
    }

    #[test]
    fn test_optimized_writer_multiple_events() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("segment");

        let config = OptimizedSegmentWriterConfig {
            num_cores: 4,
            buffer_size: 1024 * 1024,
            use_io_uring: false,
            batch_size: 100,
        };

        let mut writer = OptimizedSegmentWriter::new(path, config);

        let stream_id = StreamId::new();
        let actor_id = ActorId::new();

        for i in 0..100 {
            writer
                .append(&stream_id, &actor_id, format!("event {i}").into_bytes())
                .unwrap();
        }

        writer.seal().unwrap();

        assert_eq!(writer.events_written(), 100);
    }
}
