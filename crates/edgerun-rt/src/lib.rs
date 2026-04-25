//! Minimal epoll-based async runtime for edgerun.
//!
//! ## Architecture
//! - 1 reactor thread: epoll_wait + timer heap + waker dispatch
//! - N worker threads: poll futures from ready queue
//! - 1 blocking pool: bounded threads for `spawn_blocking`
//! - Thread-local runtime handle for free-function `spawn()`

// Modules (all functionality lives in modules, not here).
mod barrier;
mod blocking_pool;
mod io_traits;
mod join;
mod metrics;
mod mutex;
mod notify;
mod reactor;
mod reactor_fd_ready;
#[cfg(test)]
mod reactor_test;
mod ready_queue;
#[cfg(test)]
mod ready_queue_test;
mod runtime;
mod rwlock;
mod semaphore;
mod sleep_until;
mod task_map;
#[cfg(test)]
mod task_map_test;
mod tcp;
mod tcp_socket;
mod timers;
mod trace;
mod udp;
mod waker;
#[cfg(test)]
mod waker_test;
mod yield_now;
pub use join::join_internal;
mod select;
pub use select::{select_2, select_3, select_4, select_internal, Select2Enum};
mod async_fd;
pub mod broadcast;
mod buf;
mod cancellation;
mod cursor;
mod duplex;
pub mod fs;
mod io_util;
pub mod join_set;
mod latch;
pub mod mpsc;
mod once_cell;
pub mod oneshot;
pub mod process;
mod rate_limiter;
mod signal;
pub mod unbounded;
mod unix;
mod unix_dgram;
mod watch;

// ===========================================================================
// Public re-exports
// ===========================================================================

pub use std::time::Instant;

// Runtime.
pub use runtime::{spawn, spawn_blocking, Builder, JoinError, JoinHandle, Runtime, RuntimeHandle};

// Synchronization primitives — thin wrappers over `std::sync` that are
// `Send + Sync` and usable from `spawn_blocking` closures.
pub mod sync;

// Metrics.
pub use metrics::RuntimeMetrics;

// Tracing.
pub use trace::{EnterGuard, Span};

// I/O traits and extensions.
pub use io_traits::{
    poll_fn, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, PollFn, ReadLineFut, Take,
};

// TCP.
pub use tcp::{AsyncReadHalf, AsyncTcpListener, AsyncTcpStream, AsyncWriteHalf, ConnectFuture};
pub use tcp_socket::TcpSocket;

// Unix domain sockets.
pub use unix::{UnixConnectFuture, UnixListener, UnixReadHalf, UnixStream, UnixWriteHalf};
pub use unix_dgram::UnixDatagram;

// AsyncFd.
pub use async_fd::{async_fd_from_raw, pipe, AsyncFd, OwnedAsyncFd, ReadyFuture};

// I/O utilities.
pub use io_util::{copy, copy_bidirectional, empty, repeat, sink, Empty, Repeat, Sink};

// DuplexStream.
pub use duplex::DuplexStream;

// Buffered I/O.
pub use buf::{BufReader, BufWriter};

// Cursor.
pub use cursor::Cursor;

// Unix signal handling.
pub use signal::{signal, Recv, Signal, SignalKind, SignalStream};

// OnceCell.
pub use once_cell::{OnceCell, WaitUntilReady};

// Latch.
pub use latch::{Latch, WaitLatch};

// Rate limiter.
pub use rate_limiter::RateLimiter;

// UDP.
pub use udp::AsyncUdpSocket;

// Timers and signals.
pub use sleep_until::{sleep_until, timeout_at, SleepUntil, TimeoutAt};
pub use timers::{
    ctrl_c, interval, interval_at, sleep, timeout, CtrlC, Elapsed, Interval, MissedTickBehavior,
    Sleep, Timeout,
};
pub use yield_now::{yieldnow, YieldNow};

// Cancellation.
pub use cancellation::{CancellationToken, Cancelled};

// JoinSet.
pub use join_set::{JoinNext, JoinSet};

// Sync primitives.
pub use barrier::{Barrier, BarrierWaitResult};
pub use mutex::{Mutex, MutexGuard, MutexLockFuture};
pub use notify::{Notified, Notify};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use semaphore::{
    AcquireError as SemaphoreAcquireError, Permit, Semaphore,
    TryAcquireError as SemaphoreTryAcquireError,
};
pub use watch::{
    ClosedError as WatchClosedError, Receiver as WatchReceiver, Sender as WatchSender,
};
