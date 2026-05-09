//! edgerun-node runtime: bare-metal async primitives.
//!
//! This crate provides executor, time, sync, and low-level transport
//! primitives. It does not grant application authority over host resources.
//! Native socket bind/listen APIs are node/runtime implementation primitives:
//! apps request capabilities from `edgerun-node`, and the node decides whether
//! a requested protocol binding is realized as a socket, browser message route,
//! mesh route, or no resource on the current host.

pub mod time;
pub use time::{Duration, Instant};

pub use runtime::{pending, run_queue, runs};

pub mod timer;
pub use timer::{TimerWheel, elapsed_since, now, set_now, sleep_ms, sleep_us};

pub mod runtime;
pub use runtime::{
    Builder, JoinError, JoinHandle, JoinSet, Runtime, RuntimeHandle, block_on, noop_waker,
    shutdown, spawn, spawn_blocking, spawn_local,
};

pub mod timers;
pub use timers::{
    Elapsed, Sleep, SleepUntil, Timeout, TimeoutAt, sleep, sleep_until, timeout, timeout_at,
};

pub mod interval;
pub use interval::{Interval, MissedTickBehavior, interval, interval_at};

pub mod yield_now;
pub use yield_now::{YieldNow, yieldnow};

pub mod sync;
pub use sync::{
    AsyncMutex, AsyncMutexGuard, AsyncMutexLock, Condvar, Mutex, MutexGuard, Permit, RwLock,
    RwLockReadGuard, RwLockWriteGuard, Semaphore, SemaphoreAcquire, SemaphoreAcquireError,
    SemaphoreGuard, SemaphoreTryAcquireError, SpinLock, SpinLockGuard,
};

pub type SyncMutex<T> = Mutex<T>;
pub type SyncRwLock<T> = RwLock<T>;

pub mod notify;
pub use notify::{Notified, Notify};

pub mod cancellation;
pub use cancellation::{CancellationToken, Cancelled};

pub mod io;
pub use io::{
    AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, Cursor, IoError,
    copy, copy_bidirectional,
};

pub mod poll_fn;
pub use poll_fn::{PollFn, poll_fn};

pub mod join;
pub use join::{join2, join3, join4, join5};

pub mod join_internal {
    pub use crate::rt::join::{join2, join3, join4, join5};
}

pub mod select;
pub use select::{
    Either, Select, Select2Enum, Select3, Select4, select_2, select_3, select_4, select2,
};

pub mod channel;
pub use channel::{Receiver, RecvError, SendError, Sender, channel};

pub mod mpsc;
pub use mpsc::{Receiver as MpscReceiver, Sender as MpscSender};

pub fn mpsc_channel<T>(cap: usize) -> (MpscSender<T>, MpscReceiver<T>) {
    mpsc::mpsc_channel(cap)
}

pub fn unbounded_channel<T>() -> (MpscSender<T>, MpscReceiver<T>) {
    mpsc::mpsc_channel(0)
}

pub mod broadcast;
pub use broadcast::{Publisher, Subscriber, broadcast};

pub fn broadcast_channel<T: Clone + 'static>(initial: T) -> (Publisher<T>, Subscriber<T>) {
    let (publisher, subscriber) = broadcast::broadcast(1);
    let _ = publisher.send(initial);
    (publisher, subscriber)
}

pub mod watch;
pub use watch::watch as watch_channel;
pub use watch::{Receiver as WatchReceiver, Sender as WatchSender, watch};

pub mod task_local;
pub use task_local::TaskLocal;

pub mod lazy;
pub use lazy::{LazyStatic, OnceCell};

pub mod weak;
pub use weak::Weak;

pub mod error;
pub use error::Error;

pub mod signal;
pub use signal::{CtrlC, Signal, SignalHandler, SignalKind, alarm, ctrl_c, usr1, usr2};

pub mod serial_mux;

pub mod udp;
pub use udp::{SocketAddr, UdpError, UdpSocket};
pub type UdpAddr = SocketAddr;

pub mod tcp;
pub use tcp::{TcpError, TcpListener, TcpSocket, TcpState};

pub mod bare_async_net;
#[cfg(target_os = "none")]
pub use bare_async_net::{AsyncTcpListener, AsyncTcpStream, AsyncUdpSocket, ConnectFuture};
pub use bare_async_net::{BareNetDriver, install_bare_net_driver};
#[cfg(not(target_os = "none"))]
pub mod host_async_net;
#[cfg(not(target_os = "none"))]
pub use host_async_net::{
    AsyncTcpListener, AsyncTcpStream, AsyncUdpSocket, ConnectFuture, IpProtocol, SocketCapability,
    TcpBindSpec, UdpBindSpec,
};

pub mod rng;
pub use rng::Rng;

pub mod ring;
pub use ring::RingBuffer;

pub mod log;
pub use log::Level;
