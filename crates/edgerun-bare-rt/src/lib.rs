//! edgerun-bare-rt: Bare-metal async runtime

#![no_std]

extern crate alloc;

pub mod time;
pub use time::{Duration, Instant};

pub mod executor;
pub use executor::{run_queue, pending, runs};

pub mod timer;
pub use timer::{now, set_now, elapsed_since, sleep_us, sleep_ms, TimerWheel};

pub mod runtime;
pub use runtime::{
    block_on, shutdown, spawn, spawn_blocking, Builder, JoinError, JoinHandle, JoinSet, Runtime,
    RuntimeHandle, spawn_local,
};

pub mod timers;
pub use timers::{sleep, sleep_until, Elapsed, Sleep, SleepUntil, timeout, timeout_at, Timeout, TimeoutAt};

pub mod interval;
pub use interval::{interval, interval_at, Interval, MissedTickBehavior};

pub mod yield_now;
pub use yield_now::{yieldnow, YieldNow};

pub mod sync;
pub use sync::{
    AsyncMutex, AsyncMutexLock, Condvar, Mutex, MutexGuard, Permit, RwLock, RwLockReadGuard,
    RwLockWriteGuard, Semaphore, SemaphoreAcquire, SemaphoreAcquireError, SemaphoreGuard,
    SemaphoreTryAcquireError,
};

pub type SpinLock<T> = Mutex<T>;
pub type SyncMutex<T> = Mutex<T>;
pub type SyncRwLock<T> = RwLock<T>;

pub mod notify;
pub use notify::{Notified, Notify};

pub mod cancellation;
pub use cancellation::{Cancelled, CancellationToken};

pub mod io;
pub use io::{AsyncBufRead, AsyncRead, AsyncWrite, Cursor, IoError};

pub mod poll_fn;
pub use poll_fn::{poll_fn, PollFn};

pub mod join;
pub use join::{join2, join3, join4, join5};

pub mod join_internal {
    pub use crate::join::{join2, join3, join4, join5};
}

pub mod select;
pub use select::{select_2, select_3, select_4, Either, select2, Select, Select3, Select4};

pub mod channel;
pub use channel::{channel, Receiver, RecvError, SendError, Sender};

pub mod mpsc;
pub use mpsc::{Receiver as MpscReceiver, Sender as MpscSender};

pub fn mpsc_channel<T>(cap: usize) -> (MpscSender<T>, MpscReceiver<T>) {
    mpsc::mpsc_channel(cap)
}

pub fn unbounded_channel<T>() -> (MpscSender<T>, MpscReceiver<T>) {
    mpsc::mpsc_channel(0)
}

pub mod broadcast;
pub use broadcast::{broadcast, Publisher, Subscriber};

pub fn broadcast_channel<T: Clone + 'static>(initial: T) -> (Publisher<T>, Subscriber<T>) {
    let (publisher, subscriber) = broadcast::broadcast(1);
    let _ = publisher.send(initial);
    (publisher, subscriber)
}

pub mod watch;
pub use watch::{watch, Sender as WatchSender, Receiver as WatchReceiver};
pub use watch::watch as watch_channel;

pub mod task_local;
pub use task_local::TaskLocal;

pub mod lazy;
pub use lazy::{LazyStatic, OnceCell};

pub mod weak;
pub use weak::Weak;

pub mod error;
pub use error::Error;

pub mod compat;
pub use compat::{
    Barrier, BarrierWaitResult, Dir, File, Latch, RateLimiter, RuntimeMetrics, TaskMap,
    Span, TaskMetrics, TokenBucket, WaitLatch,
};

pub mod signal;
pub use signal::{ctrl_c, alarm, usr1, usr2, CtrlC, Signal, SignalHandler, SignalKind};

pub mod udp;
pub use udp::{UdpSocket, SocketAddr, UdpError};
pub type UdpAddr = SocketAddr;

pub mod tcp;
pub use tcp::{TcpSocket, TcpState, TcpError, TcpListener};

pub mod ipv4;
pub use ipv4::{Ipv4Addr, Ipv4Header, IP_VERSION, IP_DEFAULT_TTL};

pub mod rng;
pub use rng::Rng;

pub mod crc32;
pub use crc32::{Crc32, crc32};

pub mod ring;
pub use ring::RingBuffer;

pub mod storage;
pub use storage::{BlockDevice, SECTOR_SIZE};

pub mod nvme;
pub mod ata;
pub mod ahci;
pub mod fat;
pub mod pci;
pub mod virtio_net;
pub mod ip;
pub use ip::{IpStack, IpAddr, EthHeader, IpHeader, UdpHeader, TcpHeader, ArpHeader, IcmpHeader, Network, DhcpStateMachine, ETH_TYPE_IPV4, ETH_TYPE_ARP, IP_PROTO_ICMP, IP_PROTO_TCP, IP_PROTO_UDP, ICMP_ECHO_REQUEST, ICMP_ECHO_REPLY, echo_reply, ArpCache, parse_packet, ip_checksum, checksum};
pub mod dhcp;
pub use dhcp::{DhcpClient, DhcpState, DHCP_SERVER_PORT, DHCP_CLIENT_PORT};

pub mod dns;
pub use dns::{DnsQuery, DnsResponse, DnsRecord, DnsType, DnsResultCode, DNS_MAX_NAME, DnsClient};

pub mod tftp;
pub use tftp::{TftpClient, TftpState, TftpConfig};

pub mod log;
pub use log::Level;
