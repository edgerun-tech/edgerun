// SPDX-License-Identifier: GPL-2.0-only
//! Centralized process-wide I/O reactor.
//!
//! This module enforces a single submission path for disk I/O at the segment layer:
//! worker threads enqueue requests, and one dedicated reactor thread executes them.

use crossbeam_channel::{
    Receiver as CommandReceiver, RecvTimeoutError, Sender as CommandSender, bounded,
};
use io_uring::{IoUring, opcode, squeue, types};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver as TicketReceiver, Sender as TicketSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const DIRECT_ALIGNMENT: usize = 4096;
const CQ_WAIT_BATCH: usize = 8;

#[derive(Debug, Clone)]
pub struct IoReactorConfig {
    pub queue_depth: usize,
    pub batch_size: usize,
    pub max_batch_latency: Duration,
    pub use_sqpoll: bool,
    pub sqpoll_idle_ms: u32,
    pub registered_files: usize,
    pub fixed_buffer_count: usize,
    pub fixed_buffer_size: usize,
    pub use_o_direct: bool,
    pub use_o_dsync: bool,
}

impl Default for IoReactorConfig {
    fn default() -> Self {
        Self {
            queue_depth: 1024,
            batch_size: 128,
            max_batch_latency: Duration::from_micros(50),
            use_sqpoll: false,
            sqpoll_idle_ms: 2000,
            registered_files: 2048,
            fixed_buffer_count: 256,
            fixed_buffer_size: 1024 * 1024,
            use_o_direct: false,
            use_o_dsync: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IoReactorStats {
    pub ops_submitted: u64,
    pub ops_enqueued: u64,
    pub ops_completed: u64,
    pub writes_completed: u64,
    pub reads_completed: u64,
    pub fsyncs_completed: u64,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub errors: u64,
    pub avg_batch_size: f64,
    pub current_inflight: u64,
    pub max_inflight: u64,
    pub cqe_drain_calls: u64,
    pub cqe_drained_total: u64,
    pub queue_backpressure_events: u64,
}

pub struct IoTicket<T> {
    rx: TicketReceiver<io::Result<T>>,
}

impl<T> IoTicket<T> {
    pub fn wait(self) -> io::Result<T> {
        self.rx
            .recv()
            .map_err(|e| io::Error::other(e.to_string()))?
    }

    pub fn try_wait(&self) -> Option<io::Result<T>> {
        match self.rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err(io::Error::other("io ticket channel disconnected")))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IoFileHandle {
    id: u64,
}

impl IoFileHandle {
    pub fn id(&self) -> u64 {
        self.id
    }
}

enum Command {
    Open {
        path: PathBuf,
        create: bool,
        read: bool,
        write: bool,
        truncate: bool,
        direct_override: Option<bool>,
        dsync_override: Option<bool>,
        response: TicketSender<io::Result<u64>>,
    },
    Write {
        handle: u64,
        offset: u64,
        data: Vec<u8>,
        response: TicketSender<io::Result<usize>>,
    },
    WriteBatch {
        handle: u64,
        offset: u64,
        chunks: Vec<Vec<u8>>,
        response: TicketSender<io::Result<usize>>,
    },
    WriteBatchFsync {
        handle: u64,
        offset: u64,
        chunks: Vec<Vec<u8>>,
        data_only: bool,
        response: TicketSender<io::Result<usize>>,
    },
    WriteFsync {
        handle: u64,
        offset: u64,
        data: Vec<u8>,
        data_only: bool,
        response: TicketSender<io::Result<usize>>,
    },
    CheckpointFsync {
        segment_handle: u64,
        segment_offset: u64,
        segment_data: Vec<u8>,
        manifest_handle: u64,
        manifest_offset: u64,
        manifest_data: Vec<u8>,
        data_only: bool,
        response: TicketSender<io::Result<usize>>,
    },
    CheckpointBatchFsync {
        segment_handle: u64,
        segment_offset: u64,
        segment_chunks: Vec<Vec<u8>>,
        manifest_handle: u64,
        manifest_offset: u64,
        manifest_data: Vec<u8>,
        data_only: bool,
        response: TicketSender<io::Result<usize>>,
    },
    Read {
        handle: u64,
        offset: u64,
        len: usize,
        response: TicketSender<io::Result<Vec<u8>>>,
    },
    Fsync {
        handle: u64,
        data_only: bool,
        response: TicketSender<io::Result<()>>,
    },
    Truncate {
        handle: u64,
        len: u64,
        response: TicketSender<io::Result<()>>,
    },
    Preallocate {
        handle: u64,
        len: u64,
        response: TicketSender<io::Result<()>>,
    },
    Close {
        handle: u64,
    },
    Shutdown,
}

enum InFlightOp {
    Write {
        response: TicketSender<io::Result<usize>>,
        data: Vec<u8>,
    },
    WriteFixed {
        response: TicketSender<io::Result<usize>>,
        fixed_idx: usize,
        expected_len: usize,
    },
    Read {
        response: TicketSender<io::Result<Vec<u8>>>,
        buf: Vec<u8>,
    },
    ReadFixed {
        response: TicketSender<io::Result<Vec<u8>>>,
        fixed_idx: usize,
        expected_len: usize,
    },
    Fsync {
        response: TicketSender<io::Result<()>>,
    },
    LinkedWrite {
        chain_id: u64,
        data: Vec<u8>,
    },
    LinkedWriteFixed {
        chain_id: u64,
        fixed_idx: usize,
        expected_len: usize,
    },
    LinkedFsync {
        chain_id: u64,
    },
    LinkedManifestWrite {
        chain_id: u64,
        data: Vec<u8>,
    },
    LinkedManifestWriteFixed {
        chain_id: u64,
        fixed_idx: usize,
        expected_len: usize,
    },
    LinkedManifestFsync {
        chain_id: u64,
    },
    BatchWriteVectored {
        batch_id: u64,
        chunks: Vec<Vec<u8>>,
        iovecs: Vec<libc::iovec>,
        expected_len: usize,
    },
    LinkedWriteVectored {
        chain_id: u64,
        chunks: Vec<Vec<u8>>,
        iovecs: Vec<libc::iovec>,
        expected_len: usize,
    },
}

struct LinkedChain {
    response: TicketSender<io::Result<usize>>,
    checkpoint_mode: bool,
    write_result: Option<io::Result<usize>>,
    fsync_result: Option<io::Result<()>>,
    manifest_write_result: Option<io::Result<usize>>,
    manifest_fsync_result: Option<io::Result<()>>,
}

struct BatchWriteChain {
    response: TicketSender<io::Result<usize>>,
    pending_ops: usize,
    total_expected: usize,
    total_written: usize,
    error: Option<io::Error>,
}

struct FixedBufferSlot {
    storage: Vec<u8>,
    start: usize,
    len: usize,
    in_use: bool,
}

impl FixedBufferSlot {
    fn as_ptr(&self) -> *const u8 {
        // SAFETY: start is always computed to be within storage bounds.
        unsafe { self.storage.as_ptr().add(self.start) }
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        // SAFETY: start is always computed to be within storage bounds.
        unsafe { self.storage.as_mut_ptr().add(self.start) }
    }

    fn as_slice(&self) -> &[u8] {
        &self.storage[self.start..self.start + self.len]
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        let end = self.start + self.len;
        &mut self.storage[self.start..end]
    }
}

pub struct IoReactor {
    tx: CommandSender<Command>,
    stats: Arc<Mutex<IoReactorStats>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

static GLOBAL_REACTOR: OnceLock<Arc<IoReactor>> = OnceLock::new();

impl IoReactor {
    fn enqueue_ticket<T, F>(&self, build: F) -> IoTicket<T>
    where
        F: FnOnce(TicketSender<io::Result<T>>) -> Command,
    {
        let (response_tx, response_rx) = mpsc::channel();
        let command = build(response_tx.clone());
        if self.tx.send(command).is_err() {
            let _ = response_tx.send(Err(io::Error::other("io reactor is shut down")));
        }
        IoTicket { rx: response_rx }
    }

    pub fn global() -> io::Result<Arc<Self>> {
        Self::global_with_config(IoReactorConfig::default())
    }

    pub fn global_with_config(config: IoReactorConfig) -> io::Result<Arc<Self>> {
        if let Some(existing) = GLOBAL_REACTOR.get() {
            return Ok(existing.clone());
        }

        let reactor = Arc::new(Self::new(config)?);
        let _ = GLOBAL_REACTOR.set(reactor.clone());
        Ok(GLOBAL_REACTOR.get().cloned().unwrap_or(reactor))
    }

    pub fn new(config: IoReactorConfig) -> io::Result<Self> {
        let command_capacity = config
            .queue_depth
            .saturating_mul(8)
            .max(config.batch_size.saturating_mul(8))
            .max(256);
        let (tx, rx) = bounded::<Command>(command_capacity);
        let stats = Arc::new(Mutex::new(IoReactorStats::default()));
        let thread_stats = Arc::clone(&stats);

        let handle = thread::Builder::new()
            .name("storage-io-reactor".to_string())
            .spawn(move || run_reactor(rx, thread_stats, config))?;

        Ok(Self {
            tx,
            stats,
            thread: Mutex::new(Some(handle)),
        })
    }

    pub fn open_file<P: AsRef<Path>>(
        &self,
        path: P,
        create: bool,
        read: bool,
        write: bool,
        truncate: bool,
    ) -> io::Result<IoFileHandle> {
        let (response_tx, response_rx) = mpsc::channel();
        self.tx
            .send(Command::Open {
                path: path.as_ref().to_path_buf(),
                create,
                read,
                write,
                truncate,
                direct_override: None,
                dsync_override: None,
                response: response_tx,
            })
            .map_err(|e| io::Error::other(e.to_string()))?;

        let id = response_rx
            .recv()
            .map_err(|e| io::Error::other(e.to_string()))??;
        Ok(IoFileHandle { id })
    }

    pub fn open_file_buffered<P: AsRef<Path>>(
        &self,
        path: P,
        create: bool,
        read: bool,
        write: bool,
        truncate: bool,
    ) -> io::Result<IoFileHandle> {
        let (response_tx, response_rx) = mpsc::channel();
        self.tx
            .send(Command::Open {
                path: path.as_ref().to_path_buf(),
                create,
                read,
                write,
                truncate,
                direct_override: Some(false),
                dsync_override: Some(false),
                response: response_tx,
            })
            .map_err(|e| io::Error::other(e.to_string()))?;

        let id = response_rx
            .recv()
            .map_err(|e| io::Error::other(e.to_string()))??;
        Ok(IoFileHandle { id })
    }

    pub fn write(&self, handle: IoFileHandle, offset: u64, data: Vec<u8>) -> IoTicket<usize> {
        self.enqueue_ticket(|response| Command::Write {
            handle: handle.id,
            offset,
            data,
            response,
        })
    }

    pub fn write_batch(
        &self,
        handle: IoFileHandle,
        offset: u64,
        chunks: Vec<Vec<u8>>,
    ) -> IoTicket<usize> {
        self.enqueue_ticket(|response| Command::WriteBatch {
            handle: handle.id,
            offset,
            chunks,
            response,
        })
    }

    pub fn write_batch_and_fsync(
        &self,
        handle: IoFileHandle,
        offset: u64,
        chunks: Vec<Vec<u8>>,
        data_only: bool,
    ) -> IoTicket<usize> {
        self.enqueue_ticket(|response| Command::WriteBatchFsync {
            handle: handle.id,
            offset,
            chunks,
            data_only,
            response,
        })
    }

    pub fn write_and_fsync(
        &self,
        handle: IoFileHandle,
        offset: u64,
        data: Vec<u8>,
        data_only: bool,
    ) -> IoTicket<usize> {
        self.enqueue_ticket(|response| Command::WriteFsync {
            handle: handle.id,
            offset,
            data,
            data_only,
            response,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn checkpoint_write_fsync(
        &self,
        segment_handle: IoFileHandle,
        segment_offset: u64,
        segment_data: Vec<u8>,
        manifest_handle: IoFileHandle,
        manifest_offset: u64,
        manifest_data: Vec<u8>,
        data_only: bool,
    ) -> IoTicket<usize> {
        self.enqueue_ticket(|response| Command::CheckpointFsync {
            segment_handle: segment_handle.id,
            segment_offset,
            segment_data,
            manifest_handle: manifest_handle.id,
            manifest_offset,
            manifest_data,
            data_only,
            response,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn checkpoint_write_batch_fsync(
        &self,
        segment_handle: IoFileHandle,
        segment_offset: u64,
        segment_chunks: Vec<Vec<u8>>,
        manifest_handle: IoFileHandle,
        manifest_offset: u64,
        manifest_data: Vec<u8>,
        data_only: bool,
    ) -> IoTicket<usize> {
        self.enqueue_ticket(|response| Command::CheckpointBatchFsync {
            segment_handle: segment_handle.id,
            segment_offset,
            segment_chunks,
            manifest_handle: manifest_handle.id,
            manifest_offset,
            manifest_data,
            data_only,
            response,
        })
    }

    pub fn read(&self, handle: IoFileHandle, offset: u64, len: usize) -> IoTicket<Vec<u8>> {
        self.enqueue_ticket(|response| Command::Read {
            handle: handle.id,
            offset,
            len,
            response,
        })
    }

    pub fn fsync(&self, handle: IoFileHandle, data_only: bool) -> IoTicket<()> {
        self.enqueue_ticket(|response| Command::Fsync {
            handle: handle.id,
            data_only,
            response,
        })
    }

    pub fn truncate(&self, handle: IoFileHandle, len: u64) -> IoTicket<()> {
        self.enqueue_ticket(|response| Command::Truncate {
            handle: handle.id,
            len,
            response,
        })
    }

    pub fn preallocate(&self, handle: IoFileHandle, len: u64) -> IoTicket<()> {
        self.enqueue_ticket(|response| Command::Preallocate {
            handle: handle.id,
            len,
            response,
        })
    }

    pub fn close(&self, handle: IoFileHandle) {
        let _ = self.tx.send(Command::Close { handle: handle.id });
    }

    pub fn stats(&self) -> IoReactorStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Drop for IoReactor {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Ok(mut guard) = self.thread.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
    }
}

fn run_reactor(
    rx: CommandReceiver<Command>,
    stats: Arc<Mutex<IoReactorStats>>,
    config: IoReactorConfig,
) {
    let depth = u32::try_from(config.queue_depth).unwrap_or(1024).max(2);

    let mut builder = IoUring::builder();
    builder
        .setup_clamp()
        .setup_coop_taskrun()
        .setup_single_issuer();
    if config.use_sqpoll {
        builder.setup_sqpoll(config.sqpoll_idle_ms);
    }

    let mut ring = match builder.build(depth) {
        Ok(r) => r,
        Err(_) => match IoUring::new(depth) {
            Ok(r) => r,
            Err(_) => return,
        },
    };

    let mut fixed_buffers = init_fixed_buffers(&mut ring, &config);
    let file_registration = init_file_registration(&mut ring, config.registered_files);

    let mut files: HashMap<u64, File> = HashMap::new();
    let mut file_slots_by_handle: HashMap<u64, u32> = HashMap::new();

    let mut next_file_id: u64 = 1;
    let mut next_user_data: u64 = 1;
    let mut next_chain_id: u64 = 1;
    let mut next_batch_id: u64 = 1;
    let mut pending: Vec<Command> = Vec::with_capacity(config.batch_size.max(1));
    let mut inflight: HashMap<u64, InFlightOp> = HashMap::new();
    let mut chains: HashMap<u64, LinkedChain> = HashMap::new();
    let mut batch_chains: HashMap<u64, BatchWriteChain> = HashMap::new();
    let mut shutdown = false;

    while !shutdown || !inflight.is_empty() {
        pending.clear();

        if !shutdown {
            match rx.recv_timeout(config.max_batch_latency) {
                Ok(cmd) => pending.push(cmd),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => shutdown = true,
            }

            while pending.len() < config.batch_size {
                match rx.try_recv() {
                    Ok(cmd) => pending.push(cmd),
                    Err(_) => break,
                }
            }
        }

        if pending.is_empty() {
            if !inflight.is_empty() {
                let wait_for = inflight.len().clamp(1, CQ_WAIT_BATCH);
                let _ = ring.submit_and_wait(wait_for);
                drain_completions(
                    &mut ring,
                    &mut inflight,
                    &mut chains,
                    &mut batch_chains,
                    &mut fixed_buffers,
                    &stats,
                );
                update_inflight_stats(&stats, inflight.len());
            }
            continue;
        }

        let mut batch_enqueued = 0_u64;

        for cmd in pending.drain(..) {
            if matches!(cmd, Command::Shutdown) {
                shutdown = true;
                continue;
            }

            if enqueue_command(
                cmd,
                &mut ring,
                &config,
                &mut files,
                &mut file_slots_by_handle,
                file_registration,
                &mut next_file_id,
                &mut inflight,
                &mut chains,
                &mut next_user_data,
                &mut next_chain_id,
                &mut next_batch_id,
                depth as usize,
                &mut batch_chains,
                &mut fixed_buffers,
                &stats,
            ) {
                batch_enqueued += 1;
            }
        }

        if batch_enqueued > 0 {
            let _ = ring.submit();
            if let Ok(mut s) = stats.lock() {
                s.ops_submitted += batch_enqueued;
                s.ops_enqueued += batch_enqueued;
                let batch = batch_enqueued as f64;
                s.avg_batch_size = if s.avg_batch_size == 0.0 {
                    batch
                } else {
                    (s.avg_batch_size * 0.9) + (batch * 0.1)
                };
            }
            update_inflight_stats(&stats, inflight.len());
        }
    }
}

fn init_fixed_buffers(ring: &mut IoUring, config: &IoReactorConfig) -> Vec<FixedBufferSlot> {
    if config.fixed_buffer_count == 0 || config.fixed_buffer_size == 0 {
        return Vec::new();
    }

    let mut buffers = Vec::with_capacity(config.fixed_buffer_count);
    for _ in 0..config.fixed_buffer_count {
        buffers.push(create_aligned_fixed_slot(
            config.fixed_buffer_size,
            DIRECT_ALIGNMENT,
        ));
    }

    let iovecs: Vec<libc::iovec> = buffers
        .iter_mut()
        .map(|slot| libc::iovec {
            iov_base: slot.as_mut_ptr() as *mut libc::c_void,
            iov_len: slot.len,
        })
        .collect();

    // SAFETY: iovec pointers refer to fixed-size heap allocations kept alive for reactor lifetime.
    let reg_result = unsafe { ring.submitter().register_buffers(&iovecs) };
    if reg_result.is_err() {
        Vec::new()
    } else {
        buffers
    }
}

fn create_aligned_fixed_slot(size: usize, alignment: usize) -> FixedBufferSlot {
    let extra = alignment.saturating_sub(1);
    let storage = vec![0u8; size.saturating_add(extra)];
    let base = storage.as_ptr() as usize;
    let aligned = (base + extra) & !extra;
    let start = aligned.saturating_sub(base);
    FixedBufferSlot {
        storage,
        start,
        len: size,
        in_use: false,
    }
}

fn init_file_registration(ring: &mut IoUring, slots: usize) -> Option<usize> {
    if slots == 0 {
        return None;
    }

    let sparse = vec![-1; slots];
    if ring.submitter().register_files(&sparse).is_ok() {
        Some(slots)
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn enqueue_command(
    cmd: Command,
    ring: &mut IoUring,
    config: &IoReactorConfig,
    files: &mut HashMap<u64, File>,
    file_slots_by_handle: &mut HashMap<u64, u32>,
    file_registration: Option<usize>,
    next_file_id: &mut u64,
    inflight: &mut HashMap<u64, InFlightOp>,
    chains: &mut HashMap<u64, LinkedChain>,
    next_user_data: &mut u64,
    next_chain_id: &mut u64,
    next_batch_id: &mut u64,
    depth: usize,
    batch_chains: &mut HashMap<u64, BatchWriteChain>,
    fixed_buffers: &mut [FixedBufferSlot],
    stats: &Arc<Mutex<IoReactorStats>>,
) -> bool {
    match cmd {
        Command::Open {
            path,
            create,
            read,
            write,
            truncate,
            direct_override,
            dsync_override,
            response,
        } => {
            let result = open_with_flags(
                path,
                create,
                read,
                write,
                truncate,
                direct_override,
                dsync_override,
                config,
            )
            .and_then(|file| {
                let id = *next_file_id;
                *next_file_id += 1;

                if let Some(reg_slots) = file_registration {
                    if let Some(slot) = first_free_file_slot(file_slots_by_handle, reg_slots) {
                        let fd = file.as_raw_fd();
                        ring.submitter().register_files_update(slot, &[fd])?;
                        file_slots_by_handle.insert(id, slot);
                    }
                }

                files.insert(id, file);
                Ok(id)
            });
            let _ = response.send(result);
            false
        }
        Command::Write {
            handle,
            offset,
            data,
            response,
        } => submit_write(
            ring,
            files,
            file_slots_by_handle,
            handle,
            offset,
            data,
            response,
            inflight,
            next_user_data,
            depth,
            fixed_buffers,
            stats,
        ),
        Command::WriteBatch {
            handle,
            offset,
            chunks,
            response,
        } => submit_write_batch(
            ring,
            files,
            file_slots_by_handle,
            handle,
            offset,
            chunks,
            response,
            inflight,
            batch_chains,
            next_user_data,
            next_batch_id,
            depth,
            fixed_buffers,
            stats,
        ),
        Command::WriteBatchFsync {
            handle,
            offset,
            chunks,
            data_only,
            response,
        } => submit_linked_write_batch_fsync(
            ring,
            files,
            file_slots_by_handle,
            handle,
            offset,
            chunks,
            data_only,
            response,
            inflight,
            chains,
            batch_chains,
            next_user_data,
            next_chain_id,
            depth,
            fixed_buffers,
            stats,
        ),
        Command::WriteFsync {
            handle,
            offset,
            data,
            data_only,
            response,
        } => submit_linked_write_fsync(
            ring,
            files,
            file_slots_by_handle,
            handle,
            offset,
            data,
            data_only,
            response,
            inflight,
            chains,
            batch_chains,
            next_user_data,
            next_chain_id,
            depth,
            fixed_buffers,
            stats,
        ),
        Command::CheckpointFsync {
            segment_handle,
            segment_offset,
            segment_data,
            manifest_handle,
            manifest_offset,
            manifest_data,
            data_only,
            response,
        } => submit_linked_checkpoint_fsync(
            ring,
            files,
            file_slots_by_handle,
            segment_handle,
            segment_offset,
            segment_data,
            manifest_handle,
            manifest_offset,
            manifest_data,
            data_only,
            response,
            inflight,
            chains,
            batch_chains,
            next_user_data,
            next_chain_id,
            depth,
            fixed_buffers,
            stats,
        ),
        Command::CheckpointBatchFsync {
            segment_handle,
            segment_offset,
            segment_chunks,
            manifest_handle,
            manifest_offset,
            manifest_data,
            data_only,
            response,
        } => submit_linked_checkpoint_batch_fsync(
            ring,
            files,
            file_slots_by_handle,
            segment_handle,
            segment_offset,
            segment_chunks,
            manifest_handle,
            manifest_offset,
            manifest_data,
            data_only,
            response,
            inflight,
            chains,
            batch_chains,
            next_user_data,
            next_chain_id,
            depth,
            fixed_buffers,
            stats,
        ),
        Command::Read {
            handle,
            offset,
            len,
            response,
        } => submit_read(
            ring,
            files,
            file_slots_by_handle,
            handle,
            offset,
            len,
            response,
            inflight,
            next_user_data,
            depth,
            fixed_buffers,
            stats,
        ),
        Command::Fsync {
            handle,
            data_only,
            response,
        } => submit_fsync(
            ring,
            files,
            file_slots_by_handle,
            handle,
            data_only,
            response,
            inflight,
            next_user_data,
            depth,
            stats,
        ),
        Command::Truncate {
            handle,
            len,
            response,
        } => {
            let result = files
                .get(&handle)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "invalid handle"))
                .and_then(|f| f.set_len(len));
            let _ = response.send(result);
            false
        }
        Command::Preallocate {
            handle,
            len,
            response,
        } => {
            let result = files
                .get(&handle)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "invalid handle"))
                .and_then(|f| {
                    let ret =
                        unsafe { libc::posix_fallocate(f.as_raw_fd(), 0, len as libc::off_t) };
                    if ret == 0 {
                        Ok(())
                    } else {
                        Err(io::Error::from_raw_os_error(ret))
                    }
                });
            let _ = response.send(result);
            false
        }
        Command::Close { handle } => {
            if let Some(slot) = file_slots_by_handle.remove(&handle) {
                let _ = ring.submitter().register_files_update(slot, &[-1]);
            }
            files.remove(&handle);
            false
        }
        Command::Shutdown => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn open_with_flags(
    path: PathBuf,
    create: bool,
    read: bool,
    write: bool,
    truncate: bool,
    direct_override: Option<bool>,
    dsync_override: Option<bool>,
    config: &IoReactorConfig,
) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .create(create)
        .read(read)
        .write(write)
        .truncate(truncate);

    let mut custom_flags = 0;
    let use_o_direct = direct_override.unwrap_or(config.use_o_direct);
    if use_o_direct {
        custom_flags |= libc::O_DIRECT;
    }
    let use_o_dsync = dsync_override.unwrap_or(config.use_o_dsync);
    if use_o_dsync {
        custom_flags |= libc::O_DSYNC;
    }
    options.custom_flags(custom_flags);

    match options.open(&path) {
        Ok(file) => Ok(file),
        Err(e) if use_o_direct && e.raw_os_error() == Some(libc::EINVAL) => {
            // Fallback for filesystems/devices that reject O_DIRECT.
            let mut fallback = OpenOptions::new();
            fallback
                .create(create)
                .read(read)
                .write(write)
                .truncate(truncate)
                .custom_flags(if use_o_dsync { libc::O_DSYNC } else { 0 });
            fallback.open(path)
        }
        Err(e) => Err(e),
    }
}

fn first_free_file_slot(
    file_slots_by_handle: &HashMap<u64, u32>,
    total_slots: usize,
) -> Option<u32> {
    let mut used = vec![false; total_slots];
    for slot in file_slots_by_handle.values() {
        let idx = *slot as usize;
        if idx < total_slots {
            used[idx] = true;
        }
    }

    used.iter()
        .position(|in_use| !*in_use)
        .and_then(|idx| u32::try_from(idx).ok())
}

#[allow(clippy::too_many_arguments)]
fn wait_for_capacity(
    ring: &mut IoUring,
    inflight: &mut HashMap<u64, InFlightOp>,
    chains: &mut HashMap<u64, LinkedChain>,
    batch_chains: &mut HashMap<u64, BatchWriteChain>,
    fixed_buffers: &mut [FixedBufferSlot],
    depth: usize,
    needed_slots: usize,
    stats: &Arc<Mutex<IoReactorStats>>,
) {
    while inflight.len().saturating_add(needed_slots) > depth {
        if let Ok(mut s) = stats.lock() {
            s.queue_backpressure_events += 1;
        }
        let wait_for = inflight.len().clamp(1, CQ_WAIT_BATCH);
        let _ = ring.submit_and_wait(wait_for);
        drain_completions(ring, inflight, chains, batch_chains, fixed_buffers, stats);
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_write(
    ring: &mut IoUring,
    files: &HashMap<u64, File>,
    file_slots_by_handle: &HashMap<u64, u32>,
    handle: u64,
    offset: u64,
    data: Vec<u8>,
    response: TicketSender<io::Result<usize>>,
    inflight: &mut HashMap<u64, InFlightOp>,
    next_user_data: &mut u64,
    depth: usize,
    fixed_buffers: &mut [FixedBufferSlot],
    stats: &Arc<Mutex<IoReactorStats>>,
) -> bool {
    let Some(file) = files.get(&handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid handle",
        )));
        return false;
    };

    let fd_use_fixed = file_slots_by_handle
        .get(&handle)
        .map(|slot| types::Fixed(*slot));

    if data.len() > u32::MAX as usize {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "write larger than io_uring opcode limit",
        )));
        return false;
    }

    let fixed_candidate = allocate_fixed_buffer(fixed_buffers, data.len());
    let mut local_chains: HashMap<u64, LinkedChain> = HashMap::new();
    let mut local_batch_chains: HashMap<u64, BatchWriteChain> = HashMap::new();
    let mut response_opt = Some(response);
    let mut data_opt = Some(data);

    loop {
        wait_for_capacity(
            ring,
            inflight,
            &mut local_chains,
            &mut local_batch_chains,
            fixed_buffers,
            depth,
            1,
            stats,
        );

        let user_data = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let data_ref = data_opt.as_ref().expect("data present");

        let (entry, build_fixed) = if let Some(idx) = fixed_candidate {
            fixed_buffers[idx].as_mut_slice()[..data_ref.len()].copy_from_slice(data_ref);
            let fd = fd_use_fixed
                .map(|f| {
                    opcode::WriteFixed::new(
                        f,
                        fixed_buffers[idx].as_ptr(),
                        data_ref.len() as u32,
                        idx as u16,
                    )
                })
                .unwrap_or_else(|| {
                    opcode::WriteFixed::new(
                        types::Fd(file.as_raw_fd()),
                        fixed_buffers[idx].as_ptr(),
                        data_ref.len() as u32,
                        idx as u16,
                    )
                });
            (fd.offset(offset).build().user_data(user_data), true)
        } else {
            let write = fd_use_fixed
                .map(|f| opcode::Write::new(f, data_ref.as_ptr(), data_ref.len() as _))
                .unwrap_or_else(|| {
                    opcode::Write::new(
                        types::Fd(file.as_raw_fd()),
                        data_ref.as_ptr(),
                        data_ref.len() as _,
                    )
                });
            (write.offset(offset).build().user_data(user_data), false)
        };

        let mut sq = ring.submission();
        match unsafe { sq.push(&entry) } {
            Ok(()) => {
                drop(sq);
                let op = if build_fixed {
                    InFlightOp::WriteFixed {
                        response: response_opt.take().expect("response present"),
                        fixed_idx: fixed_candidate.expect("fixed idx"),
                        expected_len: data_ref.len(),
                    }
                } else {
                    InFlightOp::Write {
                        response: response_opt.take().expect("response present"),
                        data: data_opt.take().expect("data present"),
                    }
                };
                inflight.insert(user_data, op);
                return true;
            }
            Err(_) => {
                drop(sq);
                let _ = ring.submit();
                drain_completions(
                    ring,
                    inflight,
                    &mut local_chains,
                    &mut local_batch_chains,
                    fixed_buffers,
                    stats,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_write_batch(
    ring: &mut IoUring,
    files: &HashMap<u64, File>,
    file_slots_by_handle: &HashMap<u64, u32>,
    handle: u64,
    offset: u64,
    chunks: Vec<Vec<u8>>,
    response: TicketSender<io::Result<usize>>,
    inflight: &mut HashMap<u64, InFlightOp>,
    batch_chains: &mut HashMap<u64, BatchWriteChain>,
    next_user_data: &mut u64,
    next_batch_id: &mut u64,
    depth: usize,
    fixed_buffers: &mut [FixedBufferSlot],
    stats: &Arc<Mutex<IoReactorStats>>,
) -> bool {
    let Some(file) = files.get(&handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid handle",
        )));
        return false;
    };

    let non_empty: Vec<Vec<u8>> = chunks.into_iter().filter(|c| !c.is_empty()).collect();
    if non_empty.is_empty() {
        let _ = response.send(Ok(0));
        return false;
    }

    if non_empty.iter().any(|c| c.len() > u32::MAX as usize) {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "write batch chunk larger than io_uring opcode limit",
        )));
        return false;
    }

    let fd_use_fixed = file_slots_by_handle
        .get(&handle)
        .map(|slot| types::Fixed(*slot));
    let mut iovecs = Vec::with_capacity(non_empty.len());
    let mut total_expected = 0usize;
    for chunk in &non_empty {
        total_expected = total_expected.saturating_add(chunk.len());
        iovecs.push(libc::iovec {
            iov_base: chunk.as_ptr() as *mut libc::c_void,
            iov_len: chunk.len(),
        });
    }

    if iovecs.len() > u32::MAX as usize {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "write batch has too many iovecs",
        )));
        return false;
    }

    let batch_id = *next_batch_id;
    *next_batch_id = next_batch_id.saturating_add(1);
    batch_chains.insert(
        batch_id,
        BatchWriteChain {
            response,
            pending_ops: 1,
            total_expected,
            total_written: 0,
            error: None,
        },
    );

    let mut local_chains: HashMap<u64, LinkedChain> = HashMap::new();
    loop {
        wait_for_capacity(
            ring,
            inflight,
            &mut local_chains,
            batch_chains,
            fixed_buffers,
            depth,
            1,
            stats,
        );

        let user_data = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let op = fd_use_fixed
            .map(|f| opcode::Writev::new(f, iovecs.as_ptr(), iovecs.len() as _))
            .unwrap_or_else(|| {
                opcode::Writev::new(
                    types::Fd(file.as_raw_fd()),
                    iovecs.as_ptr(),
                    iovecs.len() as _,
                )
            });
        let entry = op.offset(offset).build().user_data(user_data);

        let mut sq = ring.submission();
        match unsafe { sq.push(&entry) } {
            Ok(()) => {
                drop(sq);
                inflight.insert(
                    user_data,
                    InFlightOp::BatchWriteVectored {
                        batch_id,
                        chunks: non_empty,
                        iovecs,
                        expected_len: total_expected,
                    },
                );
                return true;
            }
            Err(_) => {
                drop(sq);
                let _ = ring.submit();
                drain_completions(
                    ring,
                    inflight,
                    &mut local_chains,
                    batch_chains,
                    fixed_buffers,
                    stats,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_linked_write_batch_fsync(
    ring: &mut IoUring,
    files: &HashMap<u64, File>,
    file_slots_by_handle: &HashMap<u64, u32>,
    handle: u64,
    offset: u64,
    chunks: Vec<Vec<u8>>,
    data_only: bool,
    response: TicketSender<io::Result<usize>>,
    inflight: &mut HashMap<u64, InFlightOp>,
    chains: &mut HashMap<u64, LinkedChain>,
    batch_chains: &mut HashMap<u64, BatchWriteChain>,
    next_user_data: &mut u64,
    next_chain_id: &mut u64,
    depth: usize,
    fixed_buffers: &mut [FixedBufferSlot],
    stats: &Arc<Mutex<IoReactorStats>>,
) -> bool {
    let Some(file) = files.get(&handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid handle",
        )));
        return false;
    };

    let non_empty: Vec<Vec<u8>> = chunks.into_iter().filter(|c| !c.is_empty()).collect();
    if non_empty.is_empty() {
        let _ = response.send(Ok(0));
        return false;
    }
    if non_empty.iter().any(|c| c.len() > u32::MAX as usize) {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "write batch chunk larger than io_uring opcode limit",
        )));
        return false;
    }

    let mut iovecs = Vec::with_capacity(non_empty.len());
    let mut total_expected = 0usize;
    for chunk in &non_empty {
        total_expected = total_expected.saturating_add(chunk.len());
        iovecs.push(libc::iovec {
            iov_base: chunk.as_ptr() as *mut libc::c_void,
            iov_len: chunk.len(),
        });
    }
    if iovecs.len() > u32::MAX as usize {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "write batch has too many iovecs",
        )));
        return false;
    }

    let fd_use_fixed = file_slots_by_handle
        .get(&handle)
        .map(|slot| types::Fixed(*slot));
    let chain_id = *next_chain_id;
    *next_chain_id = next_chain_id.saturating_add(1);
    let mut response_opt = Some(response);

    loop {
        wait_for_capacity(
            ring,
            inflight,
            chains,
            batch_chains,
            fixed_buffers,
            depth,
            2,
            stats,
        );

        let write_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let fsync_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);

        let write_op = fd_use_fixed
            .map(|f| opcode::Writev::new(f, iovecs.as_ptr(), iovecs.len() as _))
            .unwrap_or_else(|| {
                opcode::Writev::new(
                    types::Fd(file.as_raw_fd()),
                    iovecs.as_ptr(),
                    iovecs.len() as _,
                )
            });
        let write_entry = write_op
            .offset(offset)
            .build()
            .flags(squeue::Flags::IO_LINK)
            .user_data(write_ud);

        let mut fsync_op = fd_use_fixed
            .map(opcode::Fsync::new)
            .unwrap_or_else(|| opcode::Fsync::new(types::Fd(file.as_raw_fd())));
        if data_only {
            fsync_op = fsync_op.flags(types::FsyncFlags::DATASYNC);
        }
        let fsync_entry = fsync_op.build().user_data(fsync_ud);

        let mut sq = ring.submission();
        match unsafe { sq.push(&write_entry) } {
            Ok(()) => match unsafe { sq.push(&fsync_entry) } {
                Ok(()) => {
                    drop(sq);
                    chains.insert(
                        chain_id,
                        LinkedChain {
                            response: response_opt.take().expect("response present"),
                            checkpoint_mode: false,
                            write_result: None,
                            fsync_result: None,
                            manifest_write_result: None,
                            manifest_fsync_result: None,
                        },
                    );
                    inflight.insert(
                        write_ud,
                        InFlightOp::LinkedWriteVectored {
                            chain_id,
                            chunks: non_empty,
                            iovecs,
                            expected_len: total_expected,
                        },
                    );
                    inflight.insert(fsync_ud, InFlightOp::LinkedFsync { chain_id });
                    return true;
                }
                Err(_) => {
                    drop(sq);
                    let _ = ring.submit();
                    drain_completions(ring, inflight, chains, batch_chains, fixed_buffers, stats);
                }
            },
            Err(_) => {
                drop(sq);
                let _ = ring.submit();
                drain_completions(ring, inflight, chains, batch_chains, fixed_buffers, stats);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_linked_write_fsync(
    ring: &mut IoUring,
    files: &HashMap<u64, File>,
    file_slots_by_handle: &HashMap<u64, u32>,
    handle: u64,
    offset: u64,
    data: Vec<u8>,
    data_only: bool,
    response: TicketSender<io::Result<usize>>,
    inflight: &mut HashMap<u64, InFlightOp>,
    chains: &mut HashMap<u64, LinkedChain>,
    batch_chains: &mut HashMap<u64, BatchWriteChain>,
    next_user_data: &mut u64,
    next_chain_id: &mut u64,
    depth: usize,
    fixed_buffers: &mut [FixedBufferSlot],
    stats: &Arc<Mutex<IoReactorStats>>,
) -> bool {
    let Some(file) = files.get(&handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid handle",
        )));
        return false;
    };

    if data.len() > u32::MAX as usize {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "write larger than io_uring opcode limit",
        )));
        return false;
    }

    let fd_use_fixed = file_slots_by_handle
        .get(&handle)
        .map(|slot| types::Fixed(*slot));

    let chain_id = *next_chain_id;
    *next_chain_id = next_chain_id.saturating_add(1);

    let fixed_candidate = allocate_fixed_buffer(fixed_buffers, data.len());
    let mut response_opt = Some(response);
    let mut data_opt = Some(data);

    loop {
        wait_for_capacity(
            ring,
            inflight,
            chains,
            batch_chains,
            fixed_buffers,
            depth,
            2,
            stats,
        );

        let write_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let fsync_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let data_ref = data_opt.as_ref().expect("data present");

        let write_entry = if let Some(idx) = fixed_candidate {
            fixed_buffers[idx].as_mut_slice()[..data_ref.len()].copy_from_slice(data_ref);
            let op = fd_use_fixed
                .map(|f| {
                    opcode::WriteFixed::new(
                        f,
                        fixed_buffers[idx].as_ptr(),
                        data_ref.len() as u32,
                        idx as u16,
                    )
                })
                .unwrap_or_else(|| {
                    opcode::WriteFixed::new(
                        types::Fd(file.as_raw_fd()),
                        fixed_buffers[idx].as_ptr(),
                        data_ref.len() as u32,
                        idx as u16,
                    )
                });
            op.offset(offset)
                .build()
                .flags(squeue::Flags::IO_LINK)
                .user_data(write_ud)
        } else {
            let op = fd_use_fixed
                .map(|f| opcode::Write::new(f, data_ref.as_ptr(), data_ref.len() as _))
                .unwrap_or_else(|| {
                    opcode::Write::new(
                        types::Fd(file.as_raw_fd()),
                        data_ref.as_ptr(),
                        data_ref.len() as _,
                    )
                });
            op.offset(offset)
                .build()
                .flags(squeue::Flags::IO_LINK)
                .user_data(write_ud)
        };

        let mut fsync_op = fd_use_fixed
            .map(opcode::Fsync::new)
            .unwrap_or_else(|| opcode::Fsync::new(types::Fd(file.as_raw_fd())));
        if data_only {
            fsync_op = fsync_op.flags(types::FsyncFlags::DATASYNC);
        }
        let fsync_entry = fsync_op.build().user_data(fsync_ud);

        let mut sq = ring.submission();
        match unsafe { sq.push(&write_entry) } {
            Ok(()) => match unsafe { sq.push(&fsync_entry) } {
                Ok(()) => {
                    drop(sq);
                    chains.insert(
                        chain_id,
                        LinkedChain {
                            response: response_opt.take().expect("response present"),
                            checkpoint_mode: false,
                            write_result: None,
                            fsync_result: None,
                            manifest_write_result: None,
                            manifest_fsync_result: None,
                        },
                    );

                    if let Some(idx) = fixed_candidate {
                        inflight.insert(
                            write_ud,
                            InFlightOp::LinkedWriteFixed {
                                chain_id,
                                fixed_idx: idx,
                                expected_len: data_ref.len(),
                            },
                        );
                    } else {
                        inflight.insert(
                            write_ud,
                            InFlightOp::LinkedWrite {
                                chain_id,
                                data: data_opt.take().expect("data present"),
                            },
                        );
                    }
                    inflight.insert(fsync_ud, InFlightOp::LinkedFsync { chain_id });
                    return true;
                }
                Err(_) => {
                    drop(sq);
                    let _ = ring.submit();
                    drain_completions(ring, inflight, chains, batch_chains, fixed_buffers, stats);
                }
            },
            Err(_) => {
                drop(sq);
                let _ = ring.submit();
                drain_completions(ring, inflight, chains, batch_chains, fixed_buffers, stats);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_linked_checkpoint_fsync(
    ring: &mut IoUring,
    files: &HashMap<u64, File>,
    file_slots_by_handle: &HashMap<u64, u32>,
    segment_handle: u64,
    segment_offset: u64,
    segment_data: Vec<u8>,
    manifest_handle: u64,
    manifest_offset: u64,
    manifest_data: Vec<u8>,
    data_only: bool,
    response: TicketSender<io::Result<usize>>,
    inflight: &mut HashMap<u64, InFlightOp>,
    chains: &mut HashMap<u64, LinkedChain>,
    batch_chains: &mut HashMap<u64, BatchWriteChain>,
    next_user_data: &mut u64,
    next_chain_id: &mut u64,
    depth: usize,
    fixed_buffers: &mut [FixedBufferSlot],
    stats: &Arc<Mutex<IoReactorStats>>,
) -> bool {
    let Some(segment_file) = files.get(&segment_handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid segment handle",
        )));
        return false;
    };
    let Some(manifest_file) = files.get(&manifest_handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid manifest handle",
        )));
        return false;
    };

    if segment_data.len() > u32::MAX as usize || manifest_data.len() > u32::MAX as usize {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "checkpoint write larger than io_uring opcode limit",
        )));
        return false;
    }

    let chain_id = *next_chain_id;
    *next_chain_id = next_chain_id.saturating_add(1);

    let segment_fd_fixed = file_slots_by_handle
        .get(&segment_handle)
        .map(|slot| types::Fixed(*slot));
    let manifest_fd_fixed = file_slots_by_handle
        .get(&manifest_handle)
        .map(|slot| types::Fixed(*slot));
    let seg_fixed_candidate = allocate_fixed_buffer(fixed_buffers, segment_data.len());
    let manifest_fixed_candidate = allocate_fixed_buffer(fixed_buffers, manifest_data.len());

    let mut response_opt = Some(response);
    let mut seg_opt = Some(segment_data);
    let mut manifest_opt = Some(manifest_data);

    loop {
        wait_for_capacity(
            ring,
            inflight,
            chains,
            batch_chains,
            fixed_buffers,
            depth,
            4,
            stats,
        );

        let seg_write_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let seg_fsync_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let manifest_write_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let manifest_fsync_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);

        let seg_ref = seg_opt.as_ref().expect("segment data present");
        let manifest_ref = manifest_opt.as_ref().expect("manifest data present");

        let seg_write = if let Some(idx) = seg_fixed_candidate {
            fixed_buffers[idx].as_mut_slice()[..seg_ref.len()].copy_from_slice(seg_ref);
            let op = segment_fd_fixed
                .map(|f| {
                    opcode::WriteFixed::new(
                        f,
                        fixed_buffers[idx].as_ptr(),
                        seg_ref.len() as u32,
                        idx as u16,
                    )
                })
                .unwrap_or_else(|| {
                    opcode::WriteFixed::new(
                        types::Fd(segment_file.as_raw_fd()),
                        fixed_buffers[idx].as_ptr(),
                        seg_ref.len() as u32,
                        idx as u16,
                    )
                });
            op.offset(segment_offset)
                .build()
                .flags(squeue::Flags::IO_LINK)
                .user_data(seg_write_ud)
        } else {
            let op = segment_fd_fixed
                .map(|f| opcode::Write::new(f, seg_ref.as_ptr(), seg_ref.len() as _))
                .unwrap_or_else(|| {
                    opcode::Write::new(
                        types::Fd(segment_file.as_raw_fd()),
                        seg_ref.as_ptr(),
                        seg_ref.len() as _,
                    )
                });
            op.offset(segment_offset)
                .build()
                .flags(squeue::Flags::IO_LINK)
                .user_data(seg_write_ud)
        };

        let mut seg_fsync_op = segment_fd_fixed
            .map(opcode::Fsync::new)
            .unwrap_or_else(|| opcode::Fsync::new(types::Fd(segment_file.as_raw_fd())));
        if data_only {
            seg_fsync_op = seg_fsync_op.flags(types::FsyncFlags::DATASYNC);
        }
        let seg_fsync = seg_fsync_op
            .build()
            .flags(squeue::Flags::IO_LINK)
            .user_data(seg_fsync_ud);

        let manifest_write = if let Some(idx) = manifest_fixed_candidate {
            fixed_buffers[idx].as_mut_slice()[..manifest_ref.len()].copy_from_slice(manifest_ref);
            let op = manifest_fd_fixed
                .map(|f| {
                    opcode::WriteFixed::new(
                        f,
                        fixed_buffers[idx].as_ptr(),
                        manifest_ref.len() as u32,
                        idx as u16,
                    )
                })
                .unwrap_or_else(|| {
                    opcode::WriteFixed::new(
                        types::Fd(manifest_file.as_raw_fd()),
                        fixed_buffers[idx].as_ptr(),
                        manifest_ref.len() as u32,
                        idx as u16,
                    )
                });
            op.offset(manifest_offset)
                .build()
                .flags(squeue::Flags::IO_LINK)
                .user_data(manifest_write_ud)
        } else {
            let op = manifest_fd_fixed
                .map(|f| opcode::Write::new(f, manifest_ref.as_ptr(), manifest_ref.len() as _))
                .unwrap_or_else(|| {
                    opcode::Write::new(
                        types::Fd(manifest_file.as_raw_fd()),
                        manifest_ref.as_ptr(),
                        manifest_ref.len() as _,
                    )
                });
            op.offset(manifest_offset)
                .build()
                .flags(squeue::Flags::IO_LINK)
                .user_data(manifest_write_ud)
        };

        let mut manifest_fsync_op = manifest_fd_fixed
            .map(opcode::Fsync::new)
            .unwrap_or_else(|| opcode::Fsync::new(types::Fd(manifest_file.as_raw_fd())));
        if data_only {
            manifest_fsync_op = manifest_fsync_op.flags(types::FsyncFlags::DATASYNC);
        }
        let manifest_fsync = manifest_fsync_op.build().user_data(manifest_fsync_ud);

        let mut sq = ring.submission();
        match unsafe { sq.push(&seg_write) } {
            Ok(()) => match unsafe { sq.push(&seg_fsync) } {
                Ok(()) => match unsafe { sq.push(&manifest_write) } {
                    Ok(()) => match unsafe { sq.push(&manifest_fsync) } {
                        Ok(()) => {
                            drop(sq);
                            chains.insert(
                                chain_id,
                                LinkedChain {
                                    response: response_opt.take().expect("response present"),
                                    checkpoint_mode: true,
                                    write_result: None,
                                    fsync_result: None,
                                    manifest_write_result: None,
                                    manifest_fsync_result: None,
                                },
                            );
                            if let Some(idx) = seg_fixed_candidate {
                                inflight.insert(
                                    seg_write_ud,
                                    InFlightOp::LinkedWriteFixed {
                                        chain_id,
                                        fixed_idx: idx,
                                        expected_len: seg_ref.len(),
                                    },
                                );
                                let _ = seg_opt.take().expect("segment data present");
                            } else {
                                inflight.insert(
                                    seg_write_ud,
                                    InFlightOp::LinkedWrite {
                                        chain_id,
                                        data: seg_opt.take().expect("segment data present"),
                                    },
                                );
                            }
                            inflight.insert(seg_fsync_ud, InFlightOp::LinkedFsync { chain_id });
                            if let Some(idx) = manifest_fixed_candidate {
                                inflight.insert(
                                    manifest_write_ud,
                                    InFlightOp::LinkedManifestWriteFixed {
                                        chain_id,
                                        fixed_idx: idx,
                                        expected_len: manifest_ref.len(),
                                    },
                                );
                                let _ = manifest_opt.take().expect("manifest data present");
                            } else {
                                inflight.insert(
                                    manifest_write_ud,
                                    InFlightOp::LinkedManifestWrite {
                                        chain_id,
                                        data: manifest_opt.take().expect("manifest data present"),
                                    },
                                );
                            }
                            inflight.insert(
                                manifest_fsync_ud,
                                InFlightOp::LinkedManifestFsync { chain_id },
                            );
                            return true;
                        }
                        Err(_) => {
                            drop(sq);
                            let _ = ring.submit();
                            drain_completions(
                                ring,
                                inflight,
                                chains,
                                batch_chains,
                                fixed_buffers,
                                stats,
                            );
                        }
                    },
                    Err(_) => {
                        drop(sq);
                        let _ = ring.submit();
                        drain_completions(
                            ring,
                            inflight,
                            chains,
                            batch_chains,
                            fixed_buffers,
                            stats,
                        );
                    }
                },
                Err(_) => {
                    drop(sq);
                    let _ = ring.submit();
                    drain_completions(ring, inflight, chains, batch_chains, fixed_buffers, stats);
                }
            },
            Err(_) => {
                drop(sq);
                let _ = ring.submit();
                drain_completions(ring, inflight, chains, batch_chains, fixed_buffers, stats);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_linked_checkpoint_batch_fsync(
    ring: &mut IoUring,
    files: &HashMap<u64, File>,
    file_slots_by_handle: &HashMap<u64, u32>,
    segment_handle: u64,
    segment_offset: u64,
    segment_chunks: Vec<Vec<u8>>,
    manifest_handle: u64,
    manifest_offset: u64,
    manifest_data: Vec<u8>,
    data_only: bool,
    response: TicketSender<io::Result<usize>>,
    inflight: &mut HashMap<u64, InFlightOp>,
    chains: &mut HashMap<u64, LinkedChain>,
    batch_chains: &mut HashMap<u64, BatchWriteChain>,
    next_user_data: &mut u64,
    next_chain_id: &mut u64,
    depth: usize,
    fixed_buffers: &mut [FixedBufferSlot],
    stats: &Arc<Mutex<IoReactorStats>>,
) -> bool {
    let Some(segment_file) = files.get(&segment_handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid segment handle",
        )));
        return false;
    };
    let Some(manifest_file) = files.get(&manifest_handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid manifest handle",
        )));
        return false;
    };

    let seg_chunks: Vec<Vec<u8>> = segment_chunks
        .into_iter()
        .filter(|c| !c.is_empty())
        .collect();
    let seg_total: usize = seg_chunks.iter().map(Vec::len).sum();
    if seg_chunks.is_empty() {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "checkpoint requires non-empty segment chunks",
        )));
        return false;
    }
    if seg_chunks.iter().any(|c| c.len() > u32::MAX as usize)
        || manifest_data.len() > u32::MAX as usize
    {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "checkpoint write larger than io_uring opcode limit",
        )));
        return false;
    }

    let mut seg_iovecs = Vec::with_capacity(seg_chunks.len());
    for chunk in &seg_chunks {
        seg_iovecs.push(libc::iovec {
            iov_base: chunk.as_ptr() as *mut libc::c_void,
            iov_len: chunk.len(),
        });
    }
    if seg_iovecs.len() > u32::MAX as usize {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "checkpoint has too many segment iovecs",
        )));
        return false;
    }

    let chain_id = *next_chain_id;
    *next_chain_id = next_chain_id.saturating_add(1);

    let segment_fd_fixed = file_slots_by_handle
        .get(&segment_handle)
        .map(|slot| types::Fixed(*slot));
    let manifest_fd_fixed = file_slots_by_handle
        .get(&manifest_handle)
        .map(|slot| types::Fixed(*slot));
    let manifest_fixed_candidate = allocate_fixed_buffer(fixed_buffers, manifest_data.len());

    let mut response_opt = Some(response);
    let mut manifest_opt = Some(manifest_data);

    loop {
        wait_for_capacity(
            ring,
            inflight,
            chains,
            batch_chains,
            fixed_buffers,
            depth,
            4,
            stats,
        );

        let seg_write_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let seg_fsync_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let manifest_write_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);
        let manifest_fsync_ud = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);

        let manifest_ref = manifest_opt.as_ref().expect("manifest data present");

        let seg_write_op = segment_fd_fixed
            .map(|f| opcode::Writev::new(f, seg_iovecs.as_ptr(), seg_iovecs.len() as _))
            .unwrap_or_else(|| {
                opcode::Writev::new(
                    types::Fd(segment_file.as_raw_fd()),
                    seg_iovecs.as_ptr(),
                    seg_iovecs.len() as _,
                )
            });
        let seg_write = seg_write_op
            .offset(segment_offset)
            .build()
            .flags(squeue::Flags::IO_LINK)
            .user_data(seg_write_ud);

        let mut seg_fsync_op = segment_fd_fixed
            .map(opcode::Fsync::new)
            .unwrap_or_else(|| opcode::Fsync::new(types::Fd(segment_file.as_raw_fd())));
        if data_only {
            seg_fsync_op = seg_fsync_op.flags(types::FsyncFlags::DATASYNC);
        }
        let seg_fsync = seg_fsync_op
            .build()
            .flags(squeue::Flags::IO_LINK)
            .user_data(seg_fsync_ud);

        let manifest_write = if let Some(idx) = manifest_fixed_candidate {
            fixed_buffers[idx].as_mut_slice()[..manifest_ref.len()].copy_from_slice(manifest_ref);
            let op = manifest_fd_fixed
                .map(|f| {
                    opcode::WriteFixed::new(
                        f,
                        fixed_buffers[idx].as_ptr(),
                        manifest_ref.len() as u32,
                        idx as u16,
                    )
                })
                .unwrap_or_else(|| {
                    opcode::WriteFixed::new(
                        types::Fd(manifest_file.as_raw_fd()),
                        fixed_buffers[idx].as_ptr(),
                        manifest_ref.len() as u32,
                        idx as u16,
                    )
                });
            op.offset(manifest_offset)
                .build()
                .flags(squeue::Flags::IO_LINK)
                .user_data(manifest_write_ud)
        } else {
            let op = manifest_fd_fixed
                .map(|f| opcode::Write::new(f, manifest_ref.as_ptr(), manifest_ref.len() as _))
                .unwrap_or_else(|| {
                    opcode::Write::new(
                        types::Fd(manifest_file.as_raw_fd()),
                        manifest_ref.as_ptr(),
                        manifest_ref.len() as _,
                    )
                });
            op.offset(manifest_offset)
                .build()
                .flags(squeue::Flags::IO_LINK)
                .user_data(manifest_write_ud)
        };

        let mut manifest_fsync_op = manifest_fd_fixed
            .map(opcode::Fsync::new)
            .unwrap_or_else(|| opcode::Fsync::new(types::Fd(manifest_file.as_raw_fd())));
        if data_only {
            manifest_fsync_op = manifest_fsync_op.flags(types::FsyncFlags::DATASYNC);
        }
        let manifest_fsync = manifest_fsync_op.build().user_data(manifest_fsync_ud);

        let mut sq = ring.submission();
        match unsafe { sq.push(&seg_write) } {
            Ok(()) => match unsafe { sq.push(&seg_fsync) } {
                Ok(()) => match unsafe { sq.push(&manifest_write) } {
                    Ok(()) => match unsafe { sq.push(&manifest_fsync) } {
                        Ok(()) => {
                            drop(sq);
                            chains.insert(
                                chain_id,
                                LinkedChain {
                                    response: response_opt.take().expect("response present"),
                                    checkpoint_mode: true,
                                    write_result: None,
                                    fsync_result: None,
                                    manifest_write_result: None,
                                    manifest_fsync_result: None,
                                },
                            );
                            inflight.insert(
                                seg_write_ud,
                                InFlightOp::LinkedWriteVectored {
                                    chain_id,
                                    chunks: seg_chunks,
                                    iovecs: seg_iovecs,
                                    expected_len: seg_total,
                                },
                            );
                            inflight.insert(seg_fsync_ud, InFlightOp::LinkedFsync { chain_id });
                            if let Some(idx) = manifest_fixed_candidate {
                                inflight.insert(
                                    manifest_write_ud,
                                    InFlightOp::LinkedManifestWriteFixed {
                                        chain_id,
                                        fixed_idx: idx,
                                        expected_len: manifest_ref.len(),
                                    },
                                );
                                let _ = manifest_opt.take().expect("manifest data present");
                            } else {
                                inflight.insert(
                                    manifest_write_ud,
                                    InFlightOp::LinkedManifestWrite {
                                        chain_id,
                                        data: manifest_opt.take().expect("manifest data present"),
                                    },
                                );
                            }
                            inflight.insert(
                                manifest_fsync_ud,
                                InFlightOp::LinkedManifestFsync { chain_id },
                            );
                            return true;
                        }
                        Err(_) => {
                            drop(sq);
                            let _ = ring.submit();
                            drain_completions(
                                ring,
                                inflight,
                                chains,
                                batch_chains,
                                fixed_buffers,
                                stats,
                            );
                        }
                    },
                    Err(_) => {
                        drop(sq);
                        let _ = ring.submit();
                        drain_completions(
                            ring,
                            inflight,
                            chains,
                            batch_chains,
                            fixed_buffers,
                            stats,
                        );
                    }
                },
                Err(_) => {
                    drop(sq);
                    let _ = ring.submit();
                    drain_completions(ring, inflight, chains, batch_chains, fixed_buffers, stats);
                }
            },
            Err(_) => {
                drop(sq);
                let _ = ring.submit();
                drain_completions(ring, inflight, chains, batch_chains, fixed_buffers, stats);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_read(
    ring: &mut IoUring,
    files: &HashMap<u64, File>,
    file_slots_by_handle: &HashMap<u64, u32>,
    handle: u64,
    offset: u64,
    len: usize,
    response: TicketSender<io::Result<Vec<u8>>>,
    inflight: &mut HashMap<u64, InFlightOp>,
    next_user_data: &mut u64,
    depth: usize,
    fixed_buffers: &mut [FixedBufferSlot],
    stats: &Arc<Mutex<IoReactorStats>>,
) -> bool {
    let Some(file) = files.get(&handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid handle",
        )));
        return false;
    };

    if len > u32::MAX as usize {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "read larger than io_uring opcode limit",
        )));
        return false;
    }

    let fd_use_fixed = file_slots_by_handle
        .get(&handle)
        .map(|slot| types::Fixed(*slot));
    let fixed_candidate = allocate_fixed_buffer(fixed_buffers, len);
    let mut local_chains: HashMap<u64, LinkedChain> = HashMap::new();
    let mut local_batch_chains: HashMap<u64, BatchWriteChain> = HashMap::new();
    let mut response_opt = Some(response);
    let mut buf_opt: Option<Vec<u8>> = None;

    loop {
        wait_for_capacity(
            ring,
            inflight,
            &mut local_chains,
            &mut local_batch_chains,
            fixed_buffers,
            depth,
            1,
            stats,
        );

        let user_data = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);

        let (entry, build_fixed) = if let Some(idx) = fixed_candidate {
            let read = fd_use_fixed
                .map(|f| {
                    opcode::ReadFixed::new(
                        f,
                        fixed_buffers[idx].as_mut_ptr(),
                        len as u32,
                        idx as u16,
                    )
                })
                .unwrap_or_else(|| {
                    opcode::ReadFixed::new(
                        types::Fd(file.as_raw_fd()),
                        fixed_buffers[idx].as_mut_ptr(),
                        len as u32,
                        idx as u16,
                    )
                });
            (read.offset(offset).build().user_data(user_data), true)
        } else {
            if buf_opt.is_none() {
                buf_opt = Some(vec![0u8; len]);
            }
            let buf = buf_opt.as_mut().expect("buffer present");
            let read = fd_use_fixed
                .map(|f| opcode::Read::new(f, buf.as_mut_ptr(), len as _))
                .unwrap_or_else(|| {
                    opcode::Read::new(types::Fd(file.as_raw_fd()), buf.as_mut_ptr(), len as _)
                });
            (read.offset(offset).build().user_data(user_data), false)
        };

        let mut sq = ring.submission();
        match unsafe { sq.push(&entry) } {
            Ok(()) => {
                drop(sq);
                let op = if build_fixed {
                    InFlightOp::ReadFixed {
                        response: response_opt.take().expect("response present"),
                        fixed_idx: fixed_candidate.expect("fixed idx"),
                        expected_len: len,
                    }
                } else {
                    InFlightOp::Read {
                        response: response_opt.take().expect("response present"),
                        buf: buf_opt.take().expect("buffer present"),
                    }
                };
                inflight.insert(user_data, op);
                return true;
            }
            Err(_) => {
                drop(sq);
                let _ = ring.submit();
                drain_completions(
                    ring,
                    inflight,
                    &mut local_chains,
                    &mut local_batch_chains,
                    fixed_buffers,
                    stats,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_fsync(
    ring: &mut IoUring,
    files: &HashMap<u64, File>,
    file_slots_by_handle: &HashMap<u64, u32>,
    handle: u64,
    data_only: bool,
    response: TicketSender<io::Result<()>>,
    inflight: &mut HashMap<u64, InFlightOp>,
    next_user_data: &mut u64,
    depth: usize,
    stats: &Arc<Mutex<IoReactorStats>>,
) -> bool {
    let Some(file) = files.get(&handle) else {
        let _ = response.send(Err(io::Error::new(
            io::ErrorKind::NotFound,
            "invalid handle",
        )));
        return false;
    };

    let fd_use_fixed = file_slots_by_handle
        .get(&handle)
        .map(|slot| types::Fixed(*slot));

    let mut local_chains: HashMap<u64, LinkedChain> = HashMap::new();
    let mut local_batch_chains: HashMap<u64, BatchWriteChain> = HashMap::new();
    let mut local_buffers: Vec<FixedBufferSlot> = Vec::new();
    let mut response_opt = Some(response);
    loop {
        while inflight.len().saturating_add(1) > depth {
            let _ = ring.submit_and_wait(1);
            drain_completions(
                ring,
                inflight,
                &mut local_chains,
                &mut local_batch_chains,
                &mut local_buffers,
                stats,
            );
        }

        let user_data = *next_user_data;
        *next_user_data = next_user_data.saturating_add(1);

        let mut op = fd_use_fixed
            .map(opcode::Fsync::new)
            .unwrap_or_else(|| opcode::Fsync::new(types::Fd(file.as_raw_fd())));
        if data_only {
            op = op.flags(types::FsyncFlags::DATASYNC);
        }
        let entry = op.build().user_data(user_data);

        let mut sq = ring.submission();
        match unsafe { sq.push(&entry) } {
            Ok(()) => {
                drop(sq);
                inflight.insert(
                    user_data,
                    InFlightOp::Fsync {
                        response: response_opt.take().expect("response present"),
                    },
                );
                return true;
            }
            Err(_) => {
                drop(sq);
                let _ = ring.submit();
                drain_completions(
                    ring,
                    inflight,
                    &mut local_chains,
                    &mut local_batch_chains,
                    &mut local_buffers,
                    stats,
                );
            }
        }
    }
}

fn allocate_fixed_buffer(fixed_buffers: &mut [FixedBufferSlot], needed: usize) -> Option<usize> {
    if needed == 0 {
        return None;
    }

    if !needed.is_multiple_of(DIRECT_ALIGNMENT) {
        return None;
    }

    fixed_buffers
        .iter_mut()
        .enumerate()
        .find(|(_, slot)| !slot.in_use && slot.len >= needed)
        .map(|(idx, slot)| {
            slot.in_use = true;
            idx
        })
}

fn release_fixed_buffer(fixed_buffers: &mut [FixedBufferSlot], idx: usize) {
    if let Some(slot) = fixed_buffers.get_mut(idx) {
        slot.in_use = false;
    }
}

fn drain_completions(
    ring: &mut IoUring,
    inflight: &mut HashMap<u64, InFlightOp>,
    chains: &mut HashMap<u64, LinkedChain>,
    batch_chains: &mut HashMap<u64, BatchWriteChain>,
    fixed_buffers: &mut [FixedBufferSlot],
    stats: &Arc<Mutex<IoReactorStats>>,
) {
    let cq = ring.completion();
    let mut drained = 0_u64;
    for cqe in cq {
        drained += 1;
        let user_data = cqe.user_data();
        let result = cqe.result();

        if let Some(op) = inflight.remove(&user_data) {
            if let Ok(mut s) = stats.lock() {
                s.ops_completed += 1;
            }

            match op {
                InFlightOp::Write { response, data } => {
                    let mapped = decode_cqe_usize(result).and_then(|bytes| {
                        if bytes > data.len() {
                            Err(io::Error::other("kernel reported invalid write size"))
                        } else {
                            Ok(bytes)
                        }
                    });
                    apply_write_stats(stats, &mapped);
                    let _ = response.send(mapped);
                }
                InFlightOp::WriteFixed {
                    response,
                    fixed_idx,
                    expected_len,
                } => {
                    release_fixed_buffer(fixed_buffers, fixed_idx);
                    let mapped = decode_cqe_usize(result).and_then(|bytes| {
                        if bytes > expected_len {
                            Err(io::Error::other("kernel reported invalid write size"))
                        } else {
                            Ok(bytes)
                        }
                    });
                    apply_write_stats(stats, &mapped);
                    let _ = response.send(mapped);
                }
                InFlightOp::Read { response, mut buf } => {
                    let mapped = decode_cqe_usize(result).map(|bytes| {
                        buf.truncate(bytes);
                        buf
                    });
                    apply_read_stats(stats, &mapped);
                    let _ = response.send(mapped);
                }
                InFlightOp::ReadFixed {
                    response,
                    fixed_idx,
                    expected_len,
                } => {
                    let mapped = decode_cqe_usize(result).map(|bytes| {
                        let clamped = bytes.min(expected_len);
                        let out = fixed_buffers[fixed_idx].as_slice()[..clamped].to_vec();
                        release_fixed_buffer(fixed_buffers, fixed_idx);
                        out
                    });
                    if mapped.is_err() {
                        release_fixed_buffer(fixed_buffers, fixed_idx);
                    }
                    apply_read_stats(stats, &mapped);
                    let _ = response.send(mapped);
                }
                InFlightOp::Fsync { response } => {
                    let mapped = decode_cqe_unit(result);
                    apply_fsync_stats(stats, &mapped);
                    let _ = response.send(mapped);
                }
                InFlightOp::LinkedWrite { chain_id, data } => {
                    let mapped = decode_cqe_usize(result).and_then(|bytes| {
                        if bytes > data.len() {
                            Err(io::Error::other("kernel reported invalid write size"))
                        } else {
                            Ok(bytes)
                        }
                    });
                    apply_write_stats(stats, &mapped);
                    if let Some(chain) = chains.get_mut(&chain_id) {
                        chain.write_result = Some(mapped);
                    }
                    complete_chain_if_ready(chains, chain_id, stats);
                }
                InFlightOp::LinkedWriteVectored {
                    chain_id,
                    chunks,
                    iovecs,
                    expected_len,
                } => {
                    let _ = iovecs.len();
                    let mapped = decode_cqe_usize(result).and_then(|bytes| {
                        let max_valid =
                            expected_len.min(chunks.iter().map(Vec::len).sum::<usize>());
                        if bytes > max_valid {
                            Err(io::Error::other("kernel reported invalid write size"))
                        } else {
                            Ok(bytes)
                        }
                    });
                    apply_write_stats(stats, &mapped);
                    if let Some(chain) = chains.get_mut(&chain_id) {
                        chain.write_result = Some(mapped);
                    }
                    complete_chain_if_ready(chains, chain_id, stats);
                }
                InFlightOp::LinkedWriteFixed {
                    chain_id,
                    fixed_idx,
                    expected_len,
                } => {
                    release_fixed_buffer(fixed_buffers, fixed_idx);
                    let mapped = decode_cqe_usize(result).and_then(|bytes| {
                        if bytes > expected_len {
                            Err(io::Error::other("kernel reported invalid write size"))
                        } else {
                            Ok(bytes)
                        }
                    });
                    apply_write_stats(stats, &mapped);
                    if let Some(chain) = chains.get_mut(&chain_id) {
                        chain.write_result = Some(mapped);
                    }
                    complete_chain_if_ready(chains, chain_id, stats);
                }
                InFlightOp::LinkedFsync { chain_id } => {
                    let mapped = decode_cqe_unit(result);
                    apply_fsync_stats(stats, &mapped);
                    if let Some(chain) = chains.get_mut(&chain_id) {
                        chain.fsync_result = Some(mapped);
                    }
                    complete_chain_if_ready(chains, chain_id, stats);
                }
                InFlightOp::LinkedManifestWrite { chain_id, data } => {
                    let mapped = decode_cqe_usize(result).and_then(|bytes| {
                        if bytes > data.len() {
                            Err(io::Error::other(
                                "kernel reported invalid manifest write size",
                            ))
                        } else {
                            Ok(bytes)
                        }
                    });
                    apply_write_stats(stats, &mapped);
                    if let Some(chain) = chains.get_mut(&chain_id) {
                        chain.manifest_write_result = Some(mapped);
                    }
                    complete_chain_if_ready(chains, chain_id, stats);
                }
                InFlightOp::LinkedManifestWriteFixed {
                    chain_id,
                    fixed_idx,
                    expected_len,
                } => {
                    release_fixed_buffer(fixed_buffers, fixed_idx);
                    let mapped = decode_cqe_usize(result).and_then(|bytes| {
                        if bytes > expected_len {
                            Err(io::Error::other(
                                "kernel reported invalid manifest write size",
                            ))
                        } else {
                            Ok(bytes)
                        }
                    });
                    apply_write_stats(stats, &mapped);
                    if let Some(chain) = chains.get_mut(&chain_id) {
                        chain.manifest_write_result = Some(mapped);
                    }
                    complete_chain_if_ready(chains, chain_id, stats);
                }
                InFlightOp::LinkedManifestFsync { chain_id } => {
                    let mapped = decode_cqe_unit(result);
                    apply_fsync_stats(stats, &mapped);
                    if let Some(chain) = chains.get_mut(&chain_id) {
                        chain.manifest_fsync_result = Some(mapped);
                    }
                    complete_chain_if_ready(chains, chain_id, stats);
                }
                InFlightOp::BatchWriteVectored {
                    batch_id,
                    chunks,
                    iovecs,
                    expected_len,
                } => {
                    let _ = iovecs.len();
                    let mapped = decode_cqe_usize(result).and_then(|bytes| {
                        let max_valid =
                            expected_len.min(chunks.iter().map(Vec::len).sum::<usize>());
                        if bytes > max_valid {
                            Err(io::Error::other("kernel reported invalid write size"))
                        } else {
                            Ok(bytes)
                        }
                    });
                    apply_write_stats(stats, &mapped);
                    if let Some(batch) = batch_chains.get_mut(&batch_id) {
                        batch.pending_ops = batch.pending_ops.saturating_sub(1);
                        match mapped {
                            Ok(bytes) => {
                                batch.total_written = batch.total_written.saturating_add(bytes)
                            }
                            Err(e) => {
                                if batch.error.is_none() {
                                    batch.error = Some(e);
                                }
                            }
                        }
                    }
                    complete_batch_if_ready(batch_chains, batch_id, stats);
                }
            }
        }
    }
    if let Ok(mut s) = stats.lock() {
        s.cqe_drain_calls += 1;
        s.cqe_drained_total += drained;
    }
}

fn update_inflight_stats(stats: &Arc<Mutex<IoReactorStats>>, inflight_len: usize) {
    if let Ok(mut s) = stats.lock() {
        s.current_inflight = inflight_len as u64;
        if s.current_inflight > s.max_inflight {
            s.max_inflight = s.current_inflight;
        }
    }
}

fn complete_chain_if_ready(
    chains: &mut HashMap<u64, LinkedChain>,
    chain_id: u64,
    stats: &Arc<Mutex<IoReactorStats>>,
) {
    let ready = chains
        .get(&chain_id)
        .map(|c| {
            if c.checkpoint_mode {
                c.write_result.is_some()
                    && c.fsync_result.is_some()
                    && c.manifest_write_result.is_some()
                    && c.manifest_fsync_result.is_some()
            } else {
                c.write_result.is_some() && c.fsync_result.is_some()
            }
        })
        .unwrap_or(false);
    if !ready {
        return;
    }

    if let Some(chain) = chains.remove(&chain_id) {
        let result = if chain.checkpoint_mode {
            match (
                chain.write_result,
                chain.fsync_result,
                chain.manifest_write_result,
                chain.manifest_fsync_result,
            ) {
                (Some(Ok(bytes)), Some(Ok(())), Some(Ok(_)), Some(Ok(()))) => Ok(bytes),
                (Some(Err(e)), _, _, _) => Err(e),
                (_, Some(Err(e)), _, _) => Err(e),
                (_, _, Some(Err(e)), _) => Err(e),
                (_, _, _, Some(Err(e))) => Err(e),
                _ => Err(io::Error::other("checkpoint chain ended in invalid state")),
            }
        } else {
            match (chain.write_result, chain.fsync_result) {
                (Some(Ok(bytes)), Some(Ok(()))) => Ok(bytes),
                (Some(Err(e)), _) => Err(e),
                (_, Some(Err(e))) => Err(e),
                _ => Err(io::Error::other("linked chain ended in invalid state")),
            }
        };

        if result.is_err() {
            if let Ok(mut s) = stats.lock() {
                s.errors += 1;
            }
        }

        let _ = chain.response.send(result);
    }
}

fn complete_batch_if_ready(
    batch_chains: &mut HashMap<u64, BatchWriteChain>,
    batch_id: u64,
    stats: &Arc<Mutex<IoReactorStats>>,
) {
    let ready = batch_chains
        .get(&batch_id)
        .map(|b| b.pending_ops == 0)
        .unwrap_or(false);
    if !ready {
        return;
    }

    if let Some(batch) = batch_chains.remove(&batch_id) {
        let result = if let Some(err) = batch.error {
            Err(err)
        } else if batch.total_written != batch.total_expected {
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                format!(
                    "short batch write completion: expected {}, got {}",
                    batch.total_expected, batch.total_written
                ),
            ))
        } else {
            Ok(batch.total_written)
        };

        if result.is_err() {
            if let Ok(mut s) = stats.lock() {
                s.errors += 1;
            }
        }

        let _ = batch.response.send(result);
    }
}

fn apply_write_stats(stats: &Arc<Mutex<IoReactorStats>>, result: &io::Result<usize>) {
    if let Ok(mut s) = stats.lock() {
        match result {
            Ok(bytes) => {
                s.writes_completed += 1;
                s.bytes_written += *bytes as u64;
            }
            Err(_) => s.errors += 1,
        }
    }
}

fn apply_read_stats(stats: &Arc<Mutex<IoReactorStats>>, result: &io::Result<Vec<u8>>) {
    if let Ok(mut s) = stats.lock() {
        match result {
            Ok(buf) => {
                s.reads_completed += 1;
                s.bytes_read += buf.len() as u64;
            }
            Err(_) => s.errors += 1,
        }
    }
}

fn apply_fsync_stats(stats: &Arc<Mutex<IoReactorStats>>, result: &io::Result<()>) {
    if let Ok(mut s) = stats.lock() {
        match result {
            Ok(()) => s.fsyncs_completed += 1,
            Err(_) => s.errors += 1,
        }
    }
}

fn decode_cqe_usize(result: i32) -> io::Result<usize> {
    if result < 0 {
        Err(io::Error::from_raw_os_error(-result))
    } else {
        Ok(result as usize)
    }
}

fn decode_cqe_unit(result: i32) -> io::Result<()> {
    if result < 0 {
        Err(io::Error::from_raw_os_error(-result))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_reactor_write_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("io-reactor.bin");
        let reactor = IoReactor::new(IoReactorConfig::default()).unwrap();
        let file = reactor.open_file(&path, true, true, true, true).unwrap();

        let payload = b"hello-reactor".to_vec();
        reactor.write(file, 0, payload.clone()).wait().unwrap();
        reactor.fsync(file, true).wait().unwrap();

        let got = reactor.read(file, 0, payload.len()).wait().unwrap();
        assert_eq!(got, payload);
    }

    #[test]
    fn test_reactor_linked_write_fsync() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("io-reactor-link.bin");
        let reactor = IoReactor::new(IoReactorConfig::default()).unwrap();
        let file = reactor.open_file(&path, true, true, true, true).unwrap();

        let payload = vec![0xAB; 4096];
        let bytes = reactor
            .write_and_fsync(file, 0, payload.clone(), true)
            .wait()
            .unwrap();
        assert_eq!(bytes, payload.len());

        let got = reactor.read(file, 0, payload.len()).wait().unwrap();
        assert_eq!(got, payload);
    }

    #[test]
    fn test_reactor_write_batch() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("io-reactor-batch.bin");
        let reactor = IoReactor::new(IoReactorConfig::default()).unwrap();
        let file = reactor.open_file(&path, true, true, true, true).unwrap();

        let chunks = vec![vec![0x11; 4096], vec![0x22; 4096], vec![0x33; 4096]];
        let expected: usize = chunks.iter().map(Vec::len).sum();

        let written = reactor.write_batch(file, 0, chunks.clone()).wait().unwrap();
        assert_eq!(written, expected);
        reactor.fsync(file, true).wait().unwrap();

        let got = reactor.read(file, 0, expected).wait().unwrap();
        let mut combined = Vec::with_capacity(expected);
        for chunk in chunks {
            combined.extend_from_slice(&chunk);
        }
        assert_eq!(got, combined);
    }

    #[test]
    fn test_reactor_checkpoint_chain() {
        let dir = TempDir::new().unwrap();
        let seg_path = dir.path().join("segment.bin");
        let manifest_path = dir.path().join("manifest.json");
        let reactor = IoReactor::new(IoReactorConfig::default()).unwrap();
        let seg = reactor
            .open_file(&seg_path, true, true, true, true)
            .unwrap();
        let manifest = reactor
            .open_file(&manifest_path, true, true, true, true)
            .unwrap();

        let seg_payload = vec![0xCD; 4096];
        let manifest_payload = br#"{\"epoch\":1}"#.to_vec();
        let written = reactor
            .checkpoint_write_fsync(
                seg,
                0,
                seg_payload.clone(),
                manifest,
                0,
                manifest_payload.clone(),
                true,
            )
            .wait()
            .unwrap();
        assert_eq!(written, seg_payload.len());

        let seg_got = reactor.read(seg, 0, seg_payload.len()).wait().unwrap();
        assert_eq!(seg_got, seg_payload);
        let manifest_got = reactor
            .read(manifest, 0, manifest_payload.len())
            .wait()
            .unwrap();
        assert_eq!(manifest_got, manifest_payload);
    }
}
