//! Minimal epoll-based async runtime for edgerun.
//!
//! ## Architecture
//! - 1 reactor thread: epoll_wait + timer heap + waker dispatch
//! - N worker threads: poll futures from ready queue
//! - 1 blocking pool: bounded threads for `spawn_blocking`
//! - Thread-local runtime handle for free-function `spawn()`

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::future::Future;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener as StdTcpL, TcpStream as StdTcp, ToSocketAddrs};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread::JoinHandle as StdJoinHandle;
use std::time::Duration;

pub use std::time::Instant;

mod notify;
mod semaphore;
mod rwlock;
mod barrier;
mod watch;
mod async_tcp;
pub mod mpsc;
pub mod oneshot;
pub mod unbounded;

pub use notify::{Notify, Notified};
pub use semaphore::{Semaphore, Permit, TryAcquireError as SemaphoreError, AcquireError};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use barrier::{Barrier, BarrierWaitResult};
pub use watch::{Sender as WatchSender, Receiver as WatchReceiver};
pub use async_tcp::{AsyncTcpStream, AsyncTcpListener, ConnectFuture, AsyncReadHalf, AsyncWriteHalf};

// ===========================================================================
// Thread-local runtime handle
// ===========================================================================

thread_local! {
    static CURRENT_RT: std::cell::RefCell<Option<Arc<RuntimeInner>>> = const { std::cell::RefCell::new(None) };
}

fn set_current_rt(rt: Arc<RuntimeInner>) {
    CURRENT_RT.with(|c| *c.borrow_mut() = Some(rt));
}

fn current_rt() -> Arc<RuntimeInner> {
    CURRENT_RT.with(|c| {
        c.borrow().clone().expect("no runtime: spawn() must be called from within a runtime")
    })
}

// ===========================================================================
// Ready queue
// ===========================================================================

struct ReadyQueue {
    q: Mutex<VecDeque<usize>>,
    cvar: Condvar,
    done: AtomicBool,
}

impl ReadyQueue {
    fn new() -> Self {
        Self { q: Mutex::new(VecDeque::new()), cvar: Condvar::new(), done: AtomicBool::new(false) }
    }
    fn push(&self, id: usize) {
        let mut q = self.q.lock().unwrap();
        q.push_back(id);
        self.cvar.notify_one();
    }
    fn pop(&self) -> Option<usize> {
        let mut q = self.q.lock().unwrap();
        loop {
            if let Some(id) = q.pop_front() { return Some(id); }
            if self.done.load(Ordering::Acquire) { return None; }
            let (q2, to) = self.cvar.wait_timeout(q, Duration::from_millis(100)).unwrap();
            q = q2;
            if to.timed_out() && self.done.load(Ordering::Acquire) && q.is_empty() { return None; }
        }
    }
    fn shutdown(&self) { self.done.store(true, Ordering::Release); self.cvar.notify_all(); }
}

// ===========================================================================
// Task store
// ===========================================================================

type PollFn = Box<dyn FnMut(&mut Context<'_>) -> bool + Send>;

struct TaskMap {
    map: Mutex<HashMap<usize, PollFn>>,
    next: AtomicUsize,
}

impl TaskMap {
    fn new() -> Self { Self { map: Mutex::new(HashMap::new()), next: AtomicUsize::new(1) } }
    fn insert(&self, f: PollFn) -> usize {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        self.map.lock().unwrap().insert(id, f);
        id
    }
    /// Take the poll function out of the map for polling (without holding the lock).
    /// Returns the task ID and the poll function. Caller must re-insert if pending.
    fn take_for_poll(&self, id: usize) -> Option<PollFn> {
        self.map.lock().unwrap().remove(&id)
    }
    fn reinsert(&self, id: usize, f: PollFn) {
        self.map.lock().unwrap().insert(id, f);
    }
}

// ===========================================================================
// Waker
// ===========================================================================

struct WData { id: usize, q: Arc<ReadyQueue> }

static VTABLE: RawWakerVTable = RawWakerVTable::new(wk_clone, wk_wake, wk_wake, wk_drop);

unsafe fn wk_clone(d: *const ()) -> RawWaker {
    // Increment the strong count so both the original and clone each own a reference
    Arc::increment_strong_count(d as *const WData);
    RawWaker::new(d, &VTABLE)
}
unsafe fn wk_wake(d: *const ()) {
    // Increment strong count before from_raw to avoid double-free when
    // the waker is also later dropped (waker clones can both wake and drop)
    Arc::increment_strong_count(d as *const WData);
    let a = Arc::from_raw(d as *const WData);
    a.q.push(a.id);
    // Arc dropped here — refcount decremented (balanced by increment above)
}
unsafe fn wk_drop(d: *const ()) {
    let _ = Arc::from_raw(d as *const WData);
}

fn make_waker(id: usize, q: Arc<ReadyQueue>) -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(Arc::into_raw(Arc::new(WData { id, q })) as *const (), &VTABLE)) }
}

// ===========================================================================
// epoll
// ===========================================================================

struct EpollFd(libc::c_int);

impl EpollFd {
    fn new() -> io::Result<Self> {
        let fd = unsafe { libc::epoll_create1(0) };
        if fd < 0 { return Err(io::Error::last_os_error()); }
        Ok(Self(fd))
    }
    fn ctl(&self, op: libc::c_int, fd: RawFd, ev: *mut libc::epoll_event) -> io::Result<()> {
        if unsafe { libc::epoll_ctl(self.0, op, fd, ev) } < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
    }
    fn wait(&self, out: &mut [libc::epoll_event], ms: i32) -> io::Result<usize> {
        let n = unsafe { libc::epoll_wait(self.0, out.as_mut_ptr(), out.len() as _, ms) };
        if n < 0 { Err(io::Error::last_os_error()) } else { Ok(n as usize) }
    }
}
impl Drop for EpollFd { fn drop(&mut self) { unsafe { libc::close(self.0) }; } }

// ===========================================================================
// FD interest
// ===========================================================================

struct FdInterest {
    read_waker: Mutex<Option<Waker>>,
    write_waker: Mutex<Option<Waker>>,
}
impl FdInterest {
    fn new() -> Self { Self { read_waker: Mutex::new(None), write_waker: Mutex::new(None) } }
    fn update(&self, epoll: &EpollFd, fd: RawFd) {
        let rp = self.read_waker.lock().unwrap().is_some();
        let wp = self.write_waker.lock().unwrap().is_some();
        let mut m = 0u32;
        if rp { m |= libc::EPOLLIN as u32 | libc::EPOLLET as u32; }
        if wp { m |= libc::EPOLLOUT as u32 | libc::EPOLLET as u32; }
        if m != 0 {
            let mut ev = libc::epoll_event { events: m as _, u64: fd as u64 };
            let _ = epoll.ctl(libc::EPOLL_CTL_MOD, fd, &mut ev);
        } else {
            let _ = epoll.ctl(libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
        }
    }
}

// ===========================================================================
// Timer heap
// ===========================================================================

struct Timer { deadline: Instant, waker: Waker }
impl Ord for Timer {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering { o.deadline.cmp(&self.deadline) }
}
impl PartialOrd for Timer {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) }
}
impl PartialEq for Timer {
    fn eq(&self, o: &Self) -> bool { self.deadline == o.deadline }
}
impl Eq for Timer {}

// ===========================================================================
// Reactor
// ===========================================================================

struct Reactor {
    epoll: EpollFd,
    fds: Mutex<HashMap<RawFd, Arc<FdInterest>>>,
    timers: Mutex<BinaryHeap<Timer>>,
    shutdown: AtomicBool,
}

impl Reactor {
    fn new() -> io::Result<Self> {
        Ok(Self { epoll: EpollFd::new()?, fds: Mutex::new(HashMap::new()), timers: Mutex::new(BinaryHeap::new()), shutdown: AtomicBool::new(false) })
    }
    fn register_fd(&self, fd: RawFd) -> io::Result<Arc<FdInterest>> {
        let mut map = self.fds.lock().unwrap();
        if let Some(s) = map.get(&fd) { return Ok(s.clone()); }
        let s = Arc::new(FdInterest::new());
        let mut ev = libc::epoll_event { events: 0, u64: fd as u64 };
        self.epoll.ctl(libc::EPOLL_CTL_ADD, fd, &mut ev)?;
        map.insert(fd, s.clone());
        Ok(s)
    }
    fn deregister_fd(&self, fd: RawFd) {
        let _ = self.epoll.ctl(libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
        self.fds.lock().unwrap().remove(&fd);
    }
    fn wait_read(&self, fd: RawFd, waker: Waker) {
        if let Some(s) = self.fds.lock().unwrap().get(&fd) {
            *s.read_waker.lock().unwrap() = Some(waker);
            s.update(&self.epoll, fd);
        }
    }
    fn wait_write(&self, fd: RawFd, waker: Waker) {
        if let Some(s) = self.fds.lock().unwrap().get(&fd) {
            *s.write_waker.lock().unwrap() = Some(waker);
            s.update(&self.epoll, fd);
        }
    }
    fn wait_connect(&self, fd: RawFd, waker: Waker) {
        // For connecting sockets, we wait for write readiness (connect complete).
        // Also register for read in case of immediate error.
        if let Some(s) = self.fds.lock().unwrap().get(&fd) {
            *s.write_waker.lock().unwrap() = Some(waker);
            s.update(&self.epoll, fd);
        }
    }
    fn clear_read(&self, fd: RawFd) {
        if let Some(s) = self.fds.lock().unwrap().get(&fd) {
            *s.read_waker.lock().unwrap() = None;
            s.update(&self.epoll, fd);
        }
    }
    fn clear_write(&self, fd: RawFd) {
        if let Some(s) = self.fds.lock().unwrap().get(&fd) {
            *s.write_waker.lock().unwrap() = None;
            s.update(&self.epoll, fd);
        }
    }
    fn register_timer(&self, deadline: Instant, waker: Waker) {
        self.timers.lock().unwrap().push(Timer { deadline, waker });
    }
    fn run(&self, _queue: &Arc<ReadyQueue>) {
        const MAX: usize = 1024;
        let mut evts: Vec<libc::epoll_event> = (0..MAX).map(|_| libc::epoll_event { events: 0, u64: 0 }).collect();
        loop {
            if self.shutdown.load(Ordering::Acquire) { break; }
            let ms = {
                let timers = self.timers.lock().unwrap();
                if let Some(t) = timers.peek() {
                    t.deadline.saturating_duration_since(Instant::now()).as_millis().min(i32::MAX as u128) as i32
                } else { 100 }
            };
            match self.epoll.wait(&mut evts, ms) {
                Ok(n) => {
                    // Fire timers
                    {
                        let mut timers = self.timers.lock().unwrap();
                        let now = Instant::now();
                        while let Some(t) = timers.peek() {
                            if t.deadline <= now {
                                let t = timers.pop().unwrap();
                                t.waker.wake();
                            } else { break; }
                        }
                    }
                    // Fire I/O
                    for i in 0..n {
                        let fd = evts[i].u64 as RawFd;
                        let bits = evts[i].events as u32;
                        if let Some(s) = self.fds.lock().unwrap().get(&fd) {
                            if (bits & (libc::EPOLLIN as u32 | libc::EPOLLHUP as u32 | libc::EPOLLERR as u32)) != 0 {
                                if let Some(w) = s.read_waker.lock().unwrap().take() { w.wake(); }
                            }
                            if (bits & (libc::EPOLLOUT as u32 | libc::EPOLLHUP as u32 | libc::EPOLLERR as u32)) != 0 {
                                if let Some(w) = s.write_waker.lock().unwrap().take() { w.wake(); }
                            }
                            s.update(&self.epoll, fd);
                            // If both wakers are gone, the fd is no longer needed —
                            // remove it from the reactor's map to prevent leaks.
                            let rp = s.read_waker.lock().unwrap().is_some();
                            let wp = s.write_waker.lock().unwrap().is_some();
                            if !rp && !wp {
                                self.deregister_fd(fd);
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => { eprintln!("edgerun-rt: epoll: {}", e); std::thread::sleep(Duration::from_millis(10)); }
            }
        }
    }
    fn shutdown(&self) { self.shutdown.store(true, Ordering::Release); }
}

// ===========================================================================
// Blocking thread pool
// ===========================================================================

struct BlockingPool {
    tx: Mutex<std::sync::mpsc::Sender<Box<dyn FnOnce() + Send>>>,
    cvar: Arc<Condvar>,
    threads: Mutex<Vec<StdJoinHandle<()>>>,
    joined: AtomicBool,
    shutdown_flag: Arc<AtomicBool>,
}

impl BlockingPool {
    fn new(size: usize) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Box<dyn FnOnce() + Send>>();
        let rx = Arc::new(Mutex::new(rx));
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let threads: Vec<_> = (0..size).map(|_| {
            let rx = Arc::clone(&rx);
            let flag = Arc::clone(&shutdown_flag);
            std::thread::spawn(move || {
                while !flag.load(Ordering::Relaxed) {
                    let job = {
                        let guard = rx.lock().unwrap();
                        guard.recv_timeout(Duration::from_millis(10))
                    };
                    match job {
                        Ok(job) => {
                            if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)) {
                                let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                                    s.to_string()
                                } else if let Some(s) = panic.downcast_ref::<String>() {
                                    s.clone()
                                } else {
                                    "unknown panic".to_string()
                                };
                                eprintln!("blocking task panicked: {}", msg);
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                // Drain remaining
                let guard = rx.lock().unwrap();
                while let Ok(job) = guard.try_recv() {
                    if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)) {
                        let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        eprintln!("blocking task panicked: {}", msg);
                    }
                }
            })
        }).collect();
        Self { tx: Mutex::new(tx), cvar: Arc::new(Condvar::new()), threads: Mutex::new(threads), joined: AtomicBool::new(false), shutdown_flag }
    }
    fn spawn<F>(&self, f: F)
    where F: FnOnce() + Send + 'static
    {
        let _ = self.tx.lock().unwrap().send(Box::new(f));
        self.cvar.notify_one();
    }
    fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::Release);
        self.cvar.notify_all();
    }
    fn join(&self) {
        if self.joined.swap(true, Ordering::AcqRel) { return; }
        for t in self.threads.lock().unwrap().drain(..) {
            let _ = t.join();
        }
    }
}

// ===========================================================================
// JoinHandle — waker-based async, Condvar for blocking_recv
// ===========================================================================

struct JoinInner<T> {
    result: Mutex<Option<Result<T, JoinError>>>,
    cvar: Condvar,
    /// Waker for async await. Set when a task polls and finds no result yet.
    waker: Mutex<Option<Waker>>,
}

pub struct JoinHandle<T> { inner: Arc<JoinInner<T>> }
impl<T> JoinHandle<T> {
    pub fn blocking_recv(self) -> Result<T, JoinError> {
        let mut guard = self.inner.result.lock().unwrap();
        while guard.is_none() {
            guard = self.inner.cvar.wait(guard).unwrap();
        }
        guard.take().unwrap()
    }
}
impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.inner.result.lock().unwrap();
        if let Some(result) = guard.take() {
            Poll::Ready(result)
        } else {
            // Register our waker so the task-completion code can wake us.
            *self.inner.waker.lock().unwrap() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
#[derive(Debug)] pub struct JoinError;
impl std::fmt::Display for JoinError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("join error") } }
impl std::error::Error for JoinError {}

// ===========================================================================
// Runtime
// ===========================================================================

struct RuntimeInner {
    reactor: Arc<Reactor>,
    tasks: Arc<TaskMap>,
    queue: Arc<ReadyQueue>,
    blocking: Arc<BlockingPool>,
    reactor_thread: Mutex<Option<StdJoinHandle<()>>>,
    worker_threads: Mutex<Vec<StdJoinHandle<()>>>,
}

impl RuntimeInner {
    fn spawn_task<F>(&self, f: F) -> JoinHandle<F::Output>
    where F: Future + Send + 'static, F::Output: Send + 'static
    {
        let inner = Arc::new(JoinInner {
            result: Mutex::new(None),
            cvar: Condvar::new(),
            waker: Mutex::new(None),
        });
        let inner2 = Arc::clone(&inner);
        let mut fut = Box::pin(f);
        let id = self.tasks.insert(Box::new(move |cx| {
            match fut.as_mut().poll(cx) {
                Poll::Ready(v) => {
                    *inner2.result.lock().unwrap() = Some(Ok(v));
                    inner2.cvar.notify_all();
                    // Wake any async task awaiting on this JoinHandle.
                    if let Some(waker) = inner2.waker.lock().unwrap().take() {
                        waker.wake();
                    }
                    false
                }
                Poll::Pending => true,
            }
        }));
        self.queue.push(id);
        JoinHandle { inner }
    }
}

pub struct Builder { workers: usize, blocking_workers: usize }
impl Builder {
    pub fn new_multi_thread() -> Self {
        Self {
            workers: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
            blocking_workers: 4,
        }
    }
    pub fn enable_all(&mut self) -> &mut Self { self }
    pub fn build(&self) -> io::Result<Runtime> {
        let queue = Arc::new(ReadyQueue::new());
        let reactor = Arc::new(Reactor::new()?);
        let tasks = Arc::new(TaskMap::new());
        let blocking = Arc::new(BlockingPool::new(self.blocking_workers));

        // Start reactor thread
        let r2 = reactor.clone();
        let q2 = queue.clone();
        let reactor_thread = std::thread::spawn(move || r2.run(&q2));

        let rt = Arc::new(RuntimeInner {
            reactor,
            tasks,
            queue,
            blocking,
            reactor_thread: Mutex::new(Some(reactor_thread)),
            worker_threads: Mutex::new(Vec::with_capacity(self.workers)),
        });

        // Start worker threads (need rt created first)
        let mut worker_threads = Vec::with_capacity(self.workers);
        for _ in 0..self.workers {
            let tasks = Arc::clone(&rt.tasks);
            let queue = Arc::clone(&rt.queue);
            let rt = Arc::clone(&rt);
            worker_threads.push(std::thread::spawn(move || {
                set_current_rt(rt);
                loop {
                    let Some(id) = queue.pop() else { break };
                    let Some(mut task) = tasks.take_for_poll(id) else { continue };
                    let w = make_waker(id, Arc::clone(&queue));
                    let mut cx = Context::from_waker(&w);
                    // Poll without holding the TaskMap lock (avoids deadlock with spawn)
                    let still_pending = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task(&mut cx)));
                    match still_pending {
                        Ok(true) => {
                            // Still pending — re-insert for next wakeup
                            tasks.reinsert(id, task);
                            // Sleep briefly to prevent tight spin-loop starving other threads
                            // (e.g., blocking pool threads that need to complete the awaited work)
                            std::thread::sleep(Duration::from_micros(100));
                        }
                        Ok(false) => {
                            // Task completed — dropped (not re-inserted)
                        }
                        Err(panic) => {
                            // Task panicked — log and drop
                            let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                                s.to_string()
                            } else if let Some(s) = panic.downcast_ref::<String>() {
                                s.clone()
                            } else {
                                "unknown panic".to_string()
                            };
                            eprintln!("task panicked: {}", msg);
                            // Task is dropped — not re-inserted
                        }
                    }
                }
            }));
        }
        *rt.worker_threads.lock().unwrap() = worker_threads;

        set_current_rt(rt.clone());

        Ok(Runtime { inner: rt })
    }
}

pub struct Runtime { inner: Arc<RuntimeInner> }
impl Runtime {
    pub fn new_multi_thread() -> Builder { Builder::new_multi_thread() }

    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where F: Future + Send + 'static, F::Output: Send + 'static
    {
        self.inner.spawn_task(f)
    }

    pub fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where F: FnOnce() -> R + Send + 'static, R: Send + 'static
    {
        let inner = Arc::new(JoinInner {
            result: Mutex::new(None),
            cvar: Condvar::new(),
            waker: Mutex::new(None),
        });
        let inner2 = Arc::clone(&inner);
        let blocking = Arc::clone(&self.inner.blocking);
        blocking.spawn(move || {
            let r = f();
            *inner2.result.lock().unwrap() = Some(Ok(r));
            inner2.cvar.notify_all();
            if let Some(waker) = inner2.waker.lock().unwrap().take() {
                waker.wake();
            }
        });
        JoinHandle { inner }
    }

    pub fn block_on<F>(&self, f: F) -> F::Output
    where F: Future + Send + 'static, F::Output: Send + 'static
    {
        set_current_rt(self.inner.clone());
        self.inner.spawn_task(f).blocking_recv().unwrap_or_else(|_| panic!("main future dropped"))
    }

    /// Graceful shutdown: signal all components and wait for threads.
    pub fn shutdown(&self) {
        self.inner.reactor.shutdown();
        self.inner.queue.shutdown();
        self.inner.blocking.shutdown();

        if let Some(handle) = self.inner.reactor_thread.lock().unwrap().take() {
            let _ = handle.join();
        }
        for t in self.inner.worker_threads.lock().unwrap().drain(..) {
            let _ = t.join();
        }
        self.inner.blocking.join();
    }

    pub fn inner(&self) -> Arc<RuntimeInner> {
        self.inner.clone()
    }
}

pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
where F: Future + Send + 'static, F::Output: Send + 'static
{
    let rt = current_rt();
    rt.spawn_task(f)
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where F: FnOnce() -> R + Send + 'static, R: Send + 'static
{
    let inner = Arc::new(JoinInner {
        result: Mutex::new(None),
        cvar: Condvar::new(),
        waker: Mutex::new(None),
    });
    let inner2 = Arc::clone(&inner);
    let rt = current_rt();
    let blocking = Arc::clone(&rt.blocking);
    blocking.spawn(move || {
        let r = f();
        *inner2.result.lock().unwrap() = Some(Ok(r));
        inner2.cvar.notify_all();
        if let Some(waker) = inner2.waker.lock().unwrap().take() {
            waker.wake();
        }
    });
    JoinHandle { inner }
}

/// Register a connecting fd with the reactor.
pub(crate) fn register_connecting_fd(fd: RawFd, waker: Waker) {
    let rt = current_rt();
    rt.reactor.wait_connect(fd, waker);
}

/// Register an existing fd for read readiness notification.
pub(crate) fn register_fd_read(fd: RawFd, waker: Waker) {
    let rt = current_rt();
    rt.reactor.wait_read(fd, waker);
}

pub fn current_handle() -> Option<Arc<RuntimeInner>> {
    CURRENT_RT.with(|c| c.borrow().clone())
}

// ===========================================================================
// Async I/O traits
// ===========================================================================

pub trait AsyncRead {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>>;
}
pub trait AsyncWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
}

pub trait AsyncReadExt: AsyncRead + Unpin {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFut<'a, Self> where Self: Sized { ReadFut { s: self, buf } }
    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadExactFut<'a, Self> where Self: Sized { ReadExactFut { s: self, buf, pos: 0 } }
}
impl<R: AsyncRead + Unpin> AsyncReadExt for R {}

pub struct ReadFut<'a, R: Unpin> { s: &'a mut R, buf: &'a mut [u8] }
impl<R: AsyncRead + Unpin> Future for ReadFut<'_, R> {
    type Output = io::Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.s).poll_read(cx, this.buf)
    }
}

pub struct ReadExactFut<'a, R: Unpin> { s: &'a mut R, buf: &'a mut [u8], pos: usize }
impl<R: AsyncRead + Unpin> Future for ReadExactFut<'_, R> {
    type Output = io::Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        while this.pos < this.buf.len() {
            let n = match Pin::new(&mut *this.s).poll_read(cx, &mut this.buf[this.pos..]) {
                Poll::Ready(Ok(0)) => return Poll::Ready(Err(io::Error::new(io::ErrorKind::UnexpectedEof, "early eof"))),
                Poll::Ready(Ok(n)) => n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            };
            this.pos += n;
        }
        Poll::Ready(Ok(()))
    }
}

pub trait AsyncWriteExt: AsyncWrite + Unpin {
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAllFut<'a, Self> where Self: Sized { WriteAllFut { s: self, buf, pos: 0 } }
    fn flush(&mut self) -> FlushFut<'_, Self> where Self: Sized { FlushFut { s: self } }
}
impl<W: AsyncWrite + Unpin> AsyncWriteExt for W {}

pub struct WriteAllFut<'a, W: Unpin> { s: &'a mut W, buf: &'a [u8], pos: usize }
impl<W: AsyncWrite + Unpin> Future for WriteAllFut<'_, W> {
    type Output = io::Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        while this.pos < this.buf.len() {
            let n = match Pin::new(&mut *this.s).poll_write(cx, &this.buf[this.pos..]) {
                Poll::Ready(Ok(n)) => n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            };
            this.pos += n;
        }
        Poll::Ready(Ok(()))
    }
}

pub struct FlushFut<'a, W: Unpin> { s: &'a mut W }
impl<W: AsyncWrite + Unpin> Future for FlushFut<'_, W> {
    type Output = io::Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.s).poll_flush(cx)
    }
}

// ===========================================================================
// TcpListener
// ===========================================================================

pub struct TcpListener { inner: StdTcpL }

pub struct BindFuture<A> { addr: Option<A>, _marker: std::marker::PhantomData<A> }
impl<A: ToSocketAddrs> Future for BindFuture<A> {
    type Output = io::Result<TcpListener>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let addr = this.addr.take().unwrap();
        match StdTcpL::bind(addr) {
            Ok(l) => { let _ = l.set_nonblocking(true); Poll::Ready(Ok(TcpListener { inner: l })) }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl TcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> BindFuture<A> {
        BindFuture { addr: Some(addr), _marker: std::marker::PhantomData }
    }
    pub fn accept(&self) -> AcceptFuture<'_> { AcceptFuture { listener: self } }
    pub fn local_addr(&self) -> io::Result<SocketAddr> { self.inner.local_addr() }
}

pub struct AcceptFuture<'a> { listener: &'a TcpListener }
impl Future for AcceptFuture<'_> {
    type Output = io::Result<(TcpStream, SocketAddr)>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let rt = current_rt();
        let fd = self.listener.inner.as_raw_fd();
        match self.listener.inner.accept() {
            Ok((s, a)) => { let _ = s.set_nonblocking(true); Poll::Ready(Ok((TcpStream::new(s), a))) }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                rt.reactor.wait_read(fd, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

// ===========================================================================
// TcpStream
// ===========================================================================

pub struct TcpStream { inner: Arc<Mutex<StdTcp>> }

impl TcpStream {
    fn new(inner: StdTcp) -> Self { Self { inner: Arc::new(Mutex::new(inner)) } }
    pub fn connect<A: ToSocketAddrs>(addr: A) -> ConnectFutureLegacy {
        ConnectFutureLegacy { addrs: addr.to_socket_addrs().ok().map(|a| a.collect::<Vec<_>>()), done: false }
    }
    pub fn peer_addr(&self) -> io::Result<SocketAddr> { self.inner.lock().unwrap().peer_addr() }
}

pub struct ConnectFutureLegacy { addrs: Option<Vec<SocketAddr>>, done: bool }
impl Future for ConnectFutureLegacy {
    type Output = io::Result<TcpStream>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.done { return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "connect failed"))); }
        this.done = true;
        let addrs = match this.addrs.take() {
            Some(a) => a,
            None => return Poll::Ready(Err(io::Error::new(io::ErrorKind::InvalidInput, "resolve failed"))),
        };
        for addr in addrs {
            if let Ok(s) = StdTcp::connect(addr) {
                let _ = s.set_nonblocking(true);
                return Poll::Ready(Ok(TcpStream::new(s)));
            }
        }
        Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "connection refused")))
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        let rt = current_rt();
        let mut inner = self.inner.lock().unwrap();
        let fd = inner.as_raw_fd();
        let mut tmp_buf = vec![0u8; buf.len()];
        match inner.read(&mut tmp_buf) {
            Ok(0) => Poll::Ready(Ok(0)),
            Ok(n) => { buf[..n].copy_from_slice(&tmp_buf[..n]); drop(inner); rt.reactor.clear_read(fd); Poll::Ready(Ok(n)) }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                drop(inner);
                rt.reactor.wait_read(fd, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let rt = current_rt();
        let mut inner = self.inner.lock().unwrap();
        let fd = inner.as_raw_fd();
        match inner.write(buf) {
            Ok(n) => { drop(inner); rt.reactor.clear_write(fd); Poll::Ready(Ok(n)) }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                drop(inner);
                rt.reactor.wait_write(fd, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut inner = self.inner.lock().unwrap();
        match inner.shutdown(std::net::Shutdown::Write) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

// ===========================================================================
// Split — Arc-based, safe
// ===========================================================================

pub struct ReadHalf { inner: Arc<Mutex<StdTcp>> }
pub struct WriteHalf { inner: Arc<Mutex<StdTcp>> }

/// Split a mutable TcpStream into read and write halves.
/// Uses Arc<Mutex<>> internally — safe, concurrent access.
pub fn split(s: &mut TcpStream) -> (ReadHalf, WriteHalf) {
    (ReadHalf { inner: Arc::clone(&s.inner) }, WriteHalf { inner: Arc::clone(&s.inner) })
}

impl AsyncRead for ReadHalf {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        let rt = current_rt();
        let mut inner = self.inner.lock().unwrap();
        let fd = inner.as_raw_fd();
        let mut tmp_buf = vec![0u8; buf.len()];
        match inner.read(&mut tmp_buf) {
            Ok(0) => Poll::Ready(Ok(0)),
            Ok(n) => { buf[..n].copy_from_slice(&tmp_buf[..n]); drop(inner); rt.reactor.clear_read(fd); Poll::Ready(Ok(n)) }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                drop(inner);
                rt.reactor.wait_read(fd, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
impl AsyncWrite for WriteHalf {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let rt = current_rt();
        let mut inner = self.inner.lock().unwrap();
        let fd = inner.as_raw_fd();
        match inner.write(buf) {
            Ok(n) => { drop(inner); rt.reactor.clear_write(fd); Poll::Ready(Ok(n)) }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                drop(inner);
                rt.reactor.wait_write(fd, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let rt = current_rt();
        let mut inner = self.inner.lock().unwrap();
        let fd = inner.as_raw_fd();
        match inner.flush() {
            Ok(()) => { drop(inner); rt.reactor.clear_write(fd); Poll::Ready(Ok(())) }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                drop(inner);
                rt.reactor.wait_write(fd, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut inner = self.inner.lock().unwrap();
        match inner.shutdown(std::net::Shutdown::Write) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

// ===========================================================================
// Timers
// ===========================================================================

pub fn sleep(d: Duration) -> Sleep { Sleep { deadline: Instant::now() + d } }
pub struct Sleep { deadline: std::time::Instant }
impl Future for Sleep {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline { Poll::Ready(()) }
        else { current_rt().reactor.register_timer(self.deadline, cx.waker().clone()); Poll::Pending }
    }
}

pub fn timeout<F>(d: Duration, f: F) -> Timeout<F> { Timeout { inner: Some(f), deadline: Instant::now() + d } }
pub struct Timeout<F> { inner: Option<F>, deadline: Instant }
impl<F: Future> Future for Timeout<F> {
    type Output = Result<F::Output, Elapsed>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let deadline = self.deadline;
        if Instant::now() >= deadline { return Poll::Ready(Err(Elapsed)); }
        let inner = unsafe { &mut self.get_unchecked_mut().inner };
        let fut = match inner { Some(f) => f, None => return Poll::Ready(Err(Elapsed)) };
        match unsafe { Pin::new_unchecked(fut) }.poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => {
                current_rt().reactor.register_timer(deadline, cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
#[derive(Debug)] pub struct Elapsed;
impl std::fmt::Display for Elapsed { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("deadline elapsed") } }
impl std::error::Error for Elapsed {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum MissedTickBehavior { Skip }

pub fn interval(d: Duration) -> Interval { Interval { d, next: Instant::now() + d, _mb: MissedTickBehavior::Skip } }
pub struct Interval { d: Duration, next: Instant, _mb: MissedTickBehavior }
impl Interval {
    pub fn set_missed_tick_behavior(&mut self, b: MissedTickBehavior) { self._mb = b; }
    pub fn tick(&mut self) -> IntervalTick<'_> { IntervalTick { interval: self } }
}
pub struct IntervalTick<'a> { interval: &'a mut Interval }
impl Future for IntervalTick<'_> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if Instant::now() >= this.interval.next {
            this.interval.next += this.interval.d;
            Poll::Ready(())
        } else {
            current_rt().reactor.register_timer(this.interval.next, cx.waker().clone());
            Poll::Pending
        }
    }
}

// ===========================================================================


// ===========================================================================
// Signal — ctrl_c
// ===========================================================================

static GOT_SIGINT: AtomicBool = AtomicBool::new(false);

pub fn ctrl_c() -> CtrlC {
    extern "C" fn handler(_: libc::c_int) { GOT_SIGINT.store(true, Ordering::Release); }
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| { unsafe { libc::signal(libc::SIGINT, handler as *const () as libc::sighandler_t); } });
    CtrlC { _p: () }
}

pub struct CtrlC { _p: () }
impl Future for CtrlC {
    type Output = io::Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if GOT_SIGINT.load(Ordering::Acquire) { Poll::Ready(Ok(())) }
        else { cx.waker().wake_by_ref(); Poll::Pending }
    }
}

// ===========================================================================
// Signal — ctrl_c
// ===========================================================================

// Tests
// ===========================================================================

mod tests {
    use super::*;

    static NOOP_WAKER: std::sync::LazyLock<Waker> = std::sync::LazyLock::new(|| {
        static VTABLE_NOOP: RawWakerVTable = RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
        const fn clone_noop(_: *const ()) -> RawWaker { RawWaker::new(std::ptr::null(), &VTABLE_NOOP) }
        const fn wake_noop(_: *const ()) {}
        const fn drop_noop(_: *const ()) {}
        unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE_NOOP)) }
    });

    fn ctx() -> Context<'static> {
        Context::from_waker(&*NOOP_WAKER)
    }

    #[test]
    fn ready_queue_push_pop_ordering() {
        let q = ReadyQueue::new();
        q.push(1); q.push(2); q.push(3);
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), Some(3));
    }

    #[test]
    fn ready_queue_shutdown() {
        let q = ReadyQueue::new();
        q.shutdown();
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn task_map_insert_take() {
        let map = TaskMap::new();
        let id = map.insert(Box::new(|_| false));
        let task = map.take_for_poll(id);
        assert!(task.is_some());
        let task = map.take_for_poll(id);
        assert!(task.is_none());
    }

    #[test]
    fn waker_enqueues() {
        let q = Arc::new(ReadyQueue::new());
        let w = make_waker(42, q.clone());
        w.wake();
        assert_eq!(q.pop(), Some(42));
    }

    #[test]
    fn epoll_create_and_drop() {
        let e = EpollFd::new().unwrap();
        drop(e);
    }

    #[test]
    fn epoll_add_and_del() {
        use std::os::unix::net::UnixStream;
        let (a, _b) = UnixStream::pair().unwrap();
        let e = EpollFd::new().unwrap();
        let fd = a.as_raw_fd();
        let mut ev = libc::epoll_event { events: 0, u64: fd as u64 };
        e.ctl(libc::EPOLL_CTL_ADD, fd, &mut ev).unwrap();
        e.ctl(libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut()).unwrap();
    }

    #[test]
    fn fd_interest_initial() {
        let fi = FdInterest::new();
        assert!(fi.read_waker.lock().unwrap().is_none());
        assert!(fi.write_waker.lock().unwrap().is_none());
    }

    // mpsc
    #[test]
    fn mpsc_send_recv() {
        let (tx, rx) = mpsc::channel::<i32>(10);
        tx.send_nowait(42).unwrap();
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(42));
    }

    #[test]
    fn mpsc_multiple() {
        let (tx, rx) = mpsc::channel::<i32>(10);
        tx.send_nowait(1).unwrap();
        tx.send_nowait(2).unwrap();
        tx.send_nowait(3).unwrap();
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(1));
        assert_eq!(rx.blocking_recv(), Some(2));
        assert_eq!(rx.blocking_recv(), Some(3));
    }

    #[test]
    fn mpsc_backpressure() {
        let (tx, _rx) = mpsc::channel::<i32>(2);
        assert!(tx.send_nowait(1).is_ok());
        assert!(tx.send_nowait(2).is_ok());
        let err = tx.send_nowait(3);
        assert!(err.is_err());
    }

    #[test]
    fn mpsc_close() {
        let (tx, rx) = mpsc::channel::<i32>(10);
        tx.send_nowait(1).unwrap();
        tx.send_nowait(2).unwrap();
        drop(tx);
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(1));
        assert_eq!(rx.blocking_recv(), Some(2));
        assert_eq!(rx.blocking_recv(), None);
    }

    #[test]
    fn mpsc_clone_send() {
        let (tx, rx) = mpsc::channel::<i32>(10);
        let tx2 = tx.clone();
        tx.send_nowait(1).unwrap();
        tx2.send_nowait(2).unwrap();
        let mut rx = rx;
        assert_eq!(rx.blocking_recv(), Some(1));
        assert_eq!(rx.blocking_recv(), Some(2));
    }

    // oneshot
    #[test]
    fn oneshot_send_recv() {
        let (tx, rx) = oneshot::channel::<i32>();
        tx.send(42).unwrap();
        let w = make_waker(1, Arc::new(ReadyQueue::new()));
        let mut cx = Context::from_waker(&w);
        let mut rx = rx;
        assert!(matches!(Pin::new(&mut rx).poll(&mut cx), Poll::Ready(Ok(42))));
    }

    #[test]
    fn oneshot_drop_sender() {
        let (tx, rx) = oneshot::channel::<i32>();
        drop(tx);
        let w = make_waker(1, Arc::new(ReadyQueue::new()));
        let mut cx = Context::from_waker(&w);
        let mut rx = rx;
        // When sender is dropped without sending, receiver gets Err immediately.
        assert!(matches!(Pin::new(&mut rx).poll(&mut cx), Poll::Ready(Err(_))));
    }

    // Sleep
    #[test]
    fn sleep_already_elapsed() {
        let mut s = Sleep { deadline: Instant::now() - Duration::from_secs(1) };
        assert_eq!(Pin::new(&mut s).poll(&mut ctx()), Poll::Ready(()));
    }

    // BlockingPool
    #[test]
    fn blocking_pool_runs_jobs() {
        let pool = BlockingPool::new(2);
        let (tx, rx) = std::sync::mpsc::channel();
        pool.spawn(move || { tx.send(42).unwrap(); });
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), 42);
        pool.shutdown();
        pool.join();
    }

    // Full runtime
    #[test]
    fn rt_spawn_and_block_on() {
        let rt = Builder::new_multi_thread().build().unwrap();
        let result = rt.block_on(async { 1 + 2 + 3 });
        assert_eq!(result, 6);
        rt.shutdown();
    }

    #[test]
    fn rt_spawn_multiple() {
        let rt = Builder::new_multi_thread().build().unwrap();
        let result = rt.block_on(async {
            let h1 = spawn(async { 10 });
            let h2 = spawn(async { 20 });
            h1.await.unwrap() + h2.await.unwrap()
        });
        assert_eq!(result, 30);
        rt.shutdown();
    }

    #[test]
    fn rt_sleep() {
        let rt = Builder::new_multi_thread().build().unwrap();
        let start = Instant::now();
        rt.block_on(async { sleep(Duration::from_millis(50)).await });
        assert!(start.elapsed() >= Duration::from_millis(40));
        rt.shutdown();
    }

    #[test]
    fn rt_sleep_with_timeout() {
        let rt = Builder::new_multi_thread().build().unwrap();
        let result = rt.block_on(async {
            timeout(Duration::from_millis(50), async {
                sleep(Duration::from_secs(10)).await;
                42
            }).await
        });
        assert!(result.is_err());
        rt.shutdown();
    }

    #[test]
    fn rt_spawn_blocking() {
        let rt = Builder::new_multi_thread().build().unwrap();
        let result = rt.block_on(async {
            let handle = spawn_blocking(|| { 42 });
            handle.await.unwrap()
        });
        assert_eq!(result, 42);
        rt.shutdown();
    }

    #[test]
    fn rt_free_spawn_works() {
        let rt = Builder::new_multi_thread().build().unwrap();
        rt.block_on(async {
            let handle = spawn(async { 100 });
            assert_eq!(handle.await.unwrap(), 100);
        });
        rt.shutdown();
    }

    #[test]
    fn tcp_listener_bind() {
        let rt = Builder::new_multi_thread().build().unwrap();
        let result = rt.block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap().port()
        });
        assert!(result > 0);
        rt.shutdown();
    }

    #[test]
    fn timer_ordering() {
        let now = Instant::now();
        let mut heap = BinaryHeap::new();
        let w1 = make_waker(1, Arc::new(ReadyQueue::new()));
        let w2 = make_waker(2, Arc::new(ReadyQueue::new()));
        let w3 = make_waker(3, Arc::new(ReadyQueue::new()));
        heap.push(Timer { deadline: now + Duration::from_millis(300), waker: w3 });
        heap.push(Timer { deadline: now + Duration::from_millis(100), waker: w1 });
        heap.push(Timer { deadline: now + Duration::from_millis(200), waker: w2 });
        assert_eq!(heap.pop().unwrap().deadline, now + Duration::from_millis(100));
        assert_eq!(heap.pop().unwrap().deadline, now + Duration::from_millis(200));
        assert_eq!(heap.pop().unwrap().deadline, now + Duration::from_millis(300));
    }
}
