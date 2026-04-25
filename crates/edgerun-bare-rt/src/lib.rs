//! edgerun-bare-rt: Bare-metal async runtime

#![no_std]

extern crate alloc;

#[macro_use]
mod log;
#[macro_use]
mod error;
#[macro_use]
mod trace;
#[macro_use]
mod metrics;

mod time;
pub use time::{Duration, Instant};
pub use log::Level;
pub use error::Error;

// Core sync primitives
mod sync_prim;
pub use sync_prim::{Condvar, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

mod oneshot;
pub use oneshot::{channel, Receiver, RecvError, Sender};

mod notify;
pub use notify::{Notified, Notify};

mod mutex;
pub use mutex::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard, MutexLockFuture};

mod latch;
pub use latch::{Latch, WaitLatch};

mod barrier;
pub use barrier::{Barrier, BarrierWait, BarrierWaitResult};

mod semaphore;
pub use semaphore::{AcquireError, Permit, Semaphore, TryAcquireError};

mod mpsc;
pub use mpsc::{channel as mpsc_channel, Receiver as MpscReceiver, Sender as MpscSender, TryRecvError};

mod unbounded;
pub use unbounded::{channel as unbounded_channel, Receiver as UnboundedReceiver, Sender as UnboundedSender};

mod broadcast;
pub use broadcast::{channel as broadcast_channel, Receiver as BroadcastReceiver, Sender as BroadcastSender};

mod watch;
pub use watch::{channel as watch_channel, ClosedError, Receiver as WatchReceiver, Sender as WatchSender};

mod cancellation;
pub use cancellation::{CancellationToken, Cancelled};

mod once_cell;
pub use once_cell::{OnceCell, WaitUntilReady};

mod join;
pub use join::{join_internal, select_2, select_3, select_4};

mod join_set;
pub use join_set::{JoinNext, JoinSet};

mod ready_queue;
mod blocking_pool;
pub use blocking_pool::{JoinError, JoinHandle, PoolError};

mod waker;
mod runtime;
pub use runtime::{spawn, spawn_blocking, Builder, Runtime, RuntimeHandle};

mod yield_now;
pub use yield_now::{yieldnow, YieldNow};

mod task_map;
pub use task_map::TaskMap;

// Platform I/O - TCP
mod tcp;
pub use tcp::{TcpSocket, SocketAddr, Error as IoError};

mod io_traits;
pub use io_traits::{
    poll_fn, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, PollFn, Take,
};

// Platform I/O - UDP
mod udp;
pub use udp::{UdpSocket as UdpSocket, SocketAddr as UdpAddr, Error as UdpError};

// Platform I/O - FS
mod fs;
pub use fs::{File, DirEntry, Dir, Error as FsError};

// Rate limiting
mod rate_limiter;
pub use rate_limiter::{RateLimiter, TokenBucket};

// Process
mod process;
pub use process::{Command, Child, ExitStatus, Error as ProcessError};

// Signal
mod signal;
pub use signal::{SignalKind, signal, ignore, default};