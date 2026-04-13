//! Minimal epoll-based async runtime for edgerun.
//!
//! ## Architecture
//! - 1 reactor thread: epoll_wait + timer heap + waker dispatch
//! - N worker threads: poll futures from ready queue
//! - 1 blocking pool: bounded threads for `spawn_blocking`
//! - Thread-local runtime handle for free-function `spawn()`

// Modules (all functionality lives in modules, not here).
mod ready_queue;
mod waker;
mod reactor;
mod task_map;
mod blocking_pool;
mod runtime;
mod io_traits;
mod tcp;
mod tcp_socket;
mod udp;
mod timers;
mod notify;
mod semaphore;
mod rwlock;
mod barrier;
mod mutex;
mod sleep_until;
mod yield_now;
mod join;
mod select;
mod cancellation;
mod unix;
mod unix_dgram;
mod async_fd;
mod io_util;
pub mod fs;
pub mod mpsc;
pub mod broadcast;
pub mod join_set;
pub mod oneshot;
pub mod unbounded;
pub mod process;
mod watch;

// ===========================================================================
// Public re-exports
// ===========================================================================

pub use std::time::Instant;

// Runtime.
pub use runtime::{Builder, Runtime, RuntimeHandle, spawn, spawn_blocking};

// I/O traits and extensions.
pub use io_traits::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

// TCP.
pub use tcp::{
    AsyncTcpStream, AsyncTcpListener, ConnectFuture, AsyncReadHalf, AsyncWriteHalf,
};
pub use tcp_socket::TcpSocket;

// Unix domain sockets.
pub use unix::{UnixStream, UnixListener, UnixReadHalf, UnixWriteHalf};
pub use unix_dgram::UnixDatagram;

// AsyncFd.
pub use async_fd::{AsyncFd, ReadyFuture, OwnedAsyncFd, async_fd_from_raw, pipe};

// I/O utilities.
pub use io_util::{copy, copy_bidirectional, empty, sink, repeat, Empty, Sink, Repeat};

// UDP.
pub use udp::AsyncUdpSocket;

// Timers and signals.
pub use timers::{
    sleep, timeout, interval, interval_at, ctrl_c, Sleep, Timeout, Elapsed, Interval,
    MissedTickBehavior, CtrlC,
};
pub use sleep_until::{sleep_until, timeout_at, SleepUntil, TimeoutAt};
pub use yield_now::{yield_now, YieldNow};

// Cancellation.
pub use cancellation::{CancellationToken, Cancelled};

// JoinSet.
pub use join_set::{JoinSet, JoinNext};

// Sync primitives.
pub use notify::{Notify, Notified};
pub use semaphore::{
    Semaphore, Permit, TryAcquireError as SemaphoreTryAcquireError,
    AcquireError as SemaphoreAcquireError,
};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use barrier::{Barrier, BarrierWaitResult};
pub use mutex::{Mutex, MutexGuard, MutexLockFuture};
pub use watch::{Sender as WatchSender, Receiver as WatchReceiver, ClosedError as WatchClosedError};
