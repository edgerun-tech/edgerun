//! edgerun-rt: Bare-metal async runtime

#![no_std]

extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std;

pub mod time;
pub use time::{Duration, Instant};

pub mod executor;
pub use executor::{pending, run_queue, runs};

pub mod timer;
pub use timer::{TimerWheel, elapsed_since, now, set_now, sleep_ms, sleep_us};

pub mod runtime;
pub use runtime::{
    Builder, JoinError, JoinHandle, JoinSet, Runtime, RuntimeHandle, block_on, shutdown, spawn,
    spawn_blocking, spawn_local,
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
    AsyncMutex, AsyncMutexLock, Condvar, Mutex, MutexGuard, Permit, RwLock, RwLockReadGuard,
    RwLockWriteGuard, Semaphore, SemaphoreAcquire, SemaphoreAcquireError, SemaphoreGuard,
    SemaphoreTryAcquireError, SpinLock, SpinLockGuard,
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
    pub use crate::join::{join2, join3, join4, join5};
}

pub mod select;
pub use select::{
    Either, Select, Select2Enum, Select3, Select4, select_2, select_3, select_4, select2,
};

pub mod channel;
pub use channel::{Receiver, RecvError, SendError, Sender, channel};

pub mod oneshot;

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

pub mod udp;
pub use udp::{SocketAddr, UdpError, UdpSocket};
pub type UdpAddr = SocketAddr;

pub mod tcp;
pub use tcp::{TcpError, TcpListener, TcpSocket, TcpState};

#[cfg(not(target_os = "none"))]
pub mod async_net;
#[cfg(not(target_os = "none"))]
pub use async_net::{AsyncTcpListener, AsyncTcpStream, AsyncUdpSocket, ConnectFuture};
#[cfg(target_os = "none")]
pub mod bare_async_net;
#[cfg(target_os = "none")]
pub use bare_async_net::{AsyncTcpListener, AsyncTcpStream, AsyncUdpSocket, ConnectFuture};

pub mod ipv4;
pub use ipv4::{IP_DEFAULT_TTL, IP_VERSION, Ipv4Addr, Ipv4Header};

pub mod rng;
pub use rng::Rng;

pub mod crc32;
pub use crc32::{Crc32, crc32};

pub mod ring;
pub use ring::RingBuffer;

pub mod storage;
pub use storage::{BlockDevice, SECTOR_SIZE};

pub mod ahci;
pub mod ata;
pub mod fat;
pub mod ip;
pub mod nvme;
pub mod pci;
pub mod virtio_net;
pub use ip::{
    ArpCache, ArpHeader, ETH_TYPE_ARP, ETH_TYPE_IPV4, EthHeader, ICMP_ECHO_REPLY,
    ICMP_ECHO_REQUEST, IP_PROTO_ICMP, IP_PROTO_TCP, IP_PROTO_UDP, IcmpHeader, IpAddr, IpHeader,
    IpStack, Network, TcpHeader, UdpHeader, checksum, echo_reply, ip_checksum, parse_packet,
};

pub mod dns;
pub use dns::{DNS_MAX_NAME, DnsClient, DnsQuery, DnsRecord, DnsResponse, DnsResultCode, DnsType};

pub mod tftp;
pub use tftp::{TftpClient, TftpConfig, TftpState};

pub mod log;
pub use log::Level;

pub mod bacnet;
pub mod ble;
pub mod can;
pub mod coap;
pub mod ftp;
pub mod grpc;
pub mod lorawan;
pub mod lwm2m;
pub mod memcached;
pub mod modbus;
pub mod mqtt;
pub mod mqttsn;
pub mod nfs;
pub mod redis;
pub mod sftp;
pub mod smb;
pub mod ssh;
pub mod websocket;
pub mod zigbee;
