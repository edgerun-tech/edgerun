//! edgerun-bare-rt: Bare-metal async runtime - core primitives only

#![no_std]

extern crate alloc;

pub mod time;
pub use time::{Duration, Instant};

pub mod timer;
pub use timer::{now, set_now, elapsed_since, sleep_us, sleep_ms, TimerWheel};

pub mod runtime;
pub use runtime::{Runtime, Builder, JoinHandle, JoinSet, spawn, spawn_local, block_on, shutdown};

pub mod timers;
pub use timers::{sleep, Elapsed, timeout, Timeout};

pub mod interval;
pub use interval::{interval, interval_at, Interval};

pub mod sync;
pub use sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard, Semaphore, SemaphoreGuard};

pub mod join;
pub use join::{join2, join3, join4, join5};

pub mod select;
pub use select::{Either, select2, Select};

pub mod channel;
pub use channel::{channel, Sender, Receiver, SendError};

pub mod mpsc;
pub use mpsc::{mpsc_channel, Sender as MpscSender, Receiver as MpscReceiver};

pub mod broadcast;
pub use broadcast::{broadcast, Publisher, Subscriber};

pub mod watch;
pub use watch::{watch, Sender as WatchSender, Receiver as WatchReceiver};

pub mod task_local;
pub use task_local::TaskLocal;

pub mod lazy;
pub use lazy::{LazyStatic, OnceCell};

pub mod weak;
pub use weak::Weak;

pub mod error;
pub use error::Error;

pub mod dhcp;
pub use dhcp::{DhcpConfig, DhcpClient, DhcpState};

pub mod tftp;
pub use tftp::{TftpConfig, TftpClient, TftpState};

pub mod log;
pub use log::Level;