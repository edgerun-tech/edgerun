//! Minimal epoll-based async runtime for Lifegraph.

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::future::Future;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener as StdTcpL, TcpStream as StdTcp, ToSocketAddrs};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::Duration;

pub use std::time::Instant;
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

type PollFn = Box<dyn FnOnce(&mut Context<'_>) -> bool + Send>;

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
    fn take(&self, id: usize) -> Option<PollFn> {
        self.map.lock().unwrap().remove(&id)
    }
}

// ===========================================================================
// Waker
// ===========================================================================

struct WData { id: usize, q: Arc<ReadyQueue> }

static VTABLE: RawWakerVTable = RawWakerVTable::new(wk_clone, wk_wake, wk_wake, wk_drop);

unsafe fn wk_clone(d: *const ()) -> RawWaker {
    let a = Arc::from_raw(d as *const WData);
    let c = Arc::clone(&a);
    std::mem::forget(a);
    RawWaker::new(Arc::into_raw(c) as *const (), &VTABLE)
}
unsafe fn wk_wake(d: *const ()) {
    let a = Arc::from_raw(d as *const WData);
    a.q.push(a.id);
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
// Manual Ord without requiring Eq on Waker
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
    fn run(&self, queue: &Arc<ReadyQueue>) {
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
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => { eprintln!("lifegraph-rt: epoll: {}", e); std::thread::sleep(Duration::from_millis(10)); }
            }
        }
    }
    fn shutdown(&self) { self.shutdown.store(true, Ordering::Release); }
}

// ===========================================================================
// JoinHandle
// ===========================================================================

pub struct JoinHandle<T> { rx: std::sync::mpsc::Receiver<Result<T, JoinError>> }
impl<T> JoinHandle<T> {
    pub fn blocking_recv(self) -> Result<T, JoinError> { self.rx.recv().unwrap_or(Err(JoinError)) }
}
impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.rx.try_recv() {
            Ok(v) => Poll::Ready(v),
            Err(std::sync::mpsc::TryRecvError::Empty) => { cx.waker().wake_by_ref(); Poll::Pending }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Poll::Ready(Err(JoinError)),
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
}

static GLOBAL: std::sync::OnceLock<Arc<RuntimeInner>> = std::sync::OnceLock::new();

fn current_rt() -> Arc<RuntimeInner> {
    GLOBAL.get().expect("no runtime").clone()
}

pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
where F: Future + Send + 'static, F::Output: Send + 'static
{
    let rt = current_rt();
    let (tx, rx) = std::sync::mpsc::channel();
    let mut fut = Box::pin(f);
    let id = rt.tasks.insert(Box::new(move |cx| {
        match fut.as_mut().poll(cx) {
            Poll::Ready(v) => { let _ = tx.send(Ok(v)); false }
            Poll::Pending => true,
        }
    }));
    rt.queue.push(id);
    JoinHandle { rx }
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where F: FnOnce() -> R + Send + 'static, R: Send + 'static
{
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || { let _ = tx.send(Ok(f())); });
    JoinHandle { rx }
}

pub struct Builder { workers: usize }
impl Builder {
    pub fn new_multi_thread() -> Self {
        Self { workers: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) }
    }
    pub fn enable_all(&mut self) -> &mut Self { self }
    pub fn build(&self) -> io::Result<Runtime> {
        let queue = Arc::new(ReadyQueue::new());
        let reactor = Arc::new(Reactor::new()?);
        let tasks = Arc::new(TaskMap::new());

        let r2 = reactor.clone();
        let q2 = queue.clone();
        std::thread::spawn(move || r2.run(&q2));

        for _ in 0..self.workers {
            let tasks = tasks.clone();
            let queue = queue.clone();
            std::thread::spawn(move || {
                loop {
                    let Some(id) = queue.pop() else { break };
                    if let Some(task) = tasks.take(id) {
                        let w = make_waker(id, queue.clone());
                        let mut cx = Context::from_waker(&w);
                        // Task returns true if pending (reactor will re-enqueue via waker)
                        // or false if done.
                        let _ = task(&mut cx);
                    }
                }
            });
        }

        let _ = GLOBAL.set(Arc::new(RuntimeInner { reactor, tasks, queue }));
        Ok(Runtime { _p: () })
    }
}

pub struct Runtime { _p: () }
impl Runtime {
    pub fn new_multi_thread() -> Builder { Builder::new_multi_thread() }
    pub fn block_on<F>(&self, f: F) -> F::Output
    where F: Future + Send + 'static, F::Output: Send + 'static
    {
        spawn(f).blocking_recv().unwrap_or_else(|_| panic!("main future dropped"))
    }
}

pub fn current_handle() -> Option<Arc<RuntimeInner>> { GLOBAL.get().cloned() }

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

// Extension traits
pub trait AsyncReadExt: AsyncRead + Unpin {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFut<'a, Self> where Self: Sized { ReadFut { s: self, buf } }
    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadExactFut<'a, Self> where Self: Sized { ReadExactFut { s: self, buf, pos: 0 } }
}
impl<R: AsyncRead + Unpin> AsyncReadExt for R {}

pub struct ReadFut<'a, R: Unpin> { s: &'a mut R, buf: &'a mut [u8] }
impl<R: AsyncRead + Unpin> Future for ReadFut<'_, R> {
    type Output = io::Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: we have &mut access through the pinned self
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
    fn flush(&mut self) -> FlushFut<Self> where Self: Sized { FlushFut { s: self } }
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

pub struct TcpStream { inner: StdTcp }
impl TcpStream {
    fn new(inner: StdTcp) -> Self { Self { inner } }
    pub fn connect<A: ToSocketAddrs>(addr: A) -> ConnectFuture {
        ConnectFuture { addrs: addr.to_socket_addrs().ok().map(|a| a.collect::<Vec<_>>()), done: false }
    }
    pub fn peer_addr(&self) -> io::Result<SocketAddr> { self.inner.peer_addr() }
}

pub struct ConnectFuture { addrs: Option<Vec<SocketAddr>>, done: bool }
impl Future for ConnectFuture {
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
        let this = unsafe { self.get_unchecked_mut() };
        let fd = this.inner.as_raw_fd();
        // Use a raw pointer to read through the immutable Pin
        let mut tmp_buf = vec![0u8; buf.len()];
        match this.inner.read(&mut tmp_buf) {
            Ok(0) => Poll::Ready(Ok(0)),
            Ok(n) => { buf[..n].copy_from_slice(&tmp_buf[..n]); rt.reactor.clear_read(fd); Poll::Ready(Ok(n)) }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
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
        let this = unsafe { self.get_unchecked_mut() };
        let fd = this.inner.as_raw_fd();
        match this.inner.write(buf) {
            Ok(n) => { rt.reactor.clear_write(fd); Poll::Ready(Ok(n)) }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                rt.reactor.wait_write(fd, cx.waker().clone());
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match unsafe { self.get_unchecked_mut() }.inner.shutdown(std::net::Shutdown::Write) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

// Split
pub struct ReadHalf<'a>(&'a mut TcpStream);
pub struct WriteHalf<'a>(&'a mut TcpStream);
pub fn split(s: &mut TcpStream) -> (ReadHalf<'_>, WriteHalf<'_>) {
    // SAFETY: We create two &mut references to the same TcpStream.
    // This is safe because ReadHalf only calls poll_read and WriteHalf only calls
    // poll_write/flush/shutdown — they don't overlap in their mutable usage.
    let ptr = s as *mut TcpStream;
    unsafe { (ReadHalf(&mut *ptr), WriteHalf(&mut *ptr)) }
}
impl AsyncRead for ReadHalf<'_> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.0).poll_read(cx, buf)
    }
}
impl AsyncWrite for WriteHalf<'_> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.0).poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.0).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        Pin::new(&mut *this.0).poll_shutdown(cx)
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
        // SAFETY: inner is Some until we resolve
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
// Channels — mpsc
// ===========================================================================

pub mod mpsc {
    use super::*;
    pub struct Sender<T> { inner: Arc<ChanInner<T>> }
    pub struct Receiver<T> { inner: Arc<ChanInner<T>> }
    struct ChanInner<T> { q: Mutex<VecDeque<T>>, waker: Mutex<Option<Waker>>, closed: AtomicBool }
    impl<T> Clone for Sender<T> { fn clone(&self) -> Self { Self { inner: self.inner.clone() } } }
    pub fn channel<T>(_cap: usize) -> (Sender<T>, Receiver<T>) {
        let inner = Arc::new(ChanInner { q: Mutex::new(VecDeque::new()), waker: Mutex::new(None), closed: AtomicBool::new(false) });
        (Sender { inner: inner.clone() }, Receiver { inner })
    }
    impl<T> Sender<T> {
        pub async fn send(&self, val: T) -> Result<(), SendError<T>> {
            if self.inner.closed.load(Ordering::Relaxed) { return Err(SendError(val)); }
            self.inner.q.lock().unwrap().push_back(val);
            if let Some(w) = self.inner.waker.lock().unwrap().take() { w.wake(); }
            Ok(())
        }
    }
    impl<T> Receiver<T> {
        pub async fn recv(&mut self) -> Option<T> { RecvFut { inner: &self.inner }.await }
        pub fn blocking_recv(&mut self) -> Option<T> {
            loop {
                if let Some(v) = self.inner.q.lock().unwrap().pop_front() { return Some(v); }
                if self.inner.closed.load(Ordering::Relaxed) { return None; }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }
    struct RecvFut<'a, T> { inner: &'a ChanInner<T> }
    impl<T> Future for RecvFut<'_, T> {
        type Output = Option<T>;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if let Some(v) = self.inner.q.lock().unwrap().pop_front() { Poll::Ready(Some(v)) }
            else if self.inner.closed.load(Ordering::Relaxed) { Poll::Ready(None) }
            else { *self.inner.waker.lock().unwrap() = Some(cx.waker().clone()); Poll::Pending }
        }
    }
    #[derive(Debug)] pub struct SendError<T>(pub T);
    impl<T> std::fmt::Display for SendError<T> { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("send error") } }
}

// ===========================================================================
// Channels — oneshot
// ===========================================================================

pub mod oneshot {
    use super::*;
    pub struct Sender<T> { inner: Option<Arc<OneInner<T>>> }
    pub struct Receiver<T> { inner: Arc<OneInner<T>> }
    struct OneInner<T> { val: Mutex<Option<T>>, waker: Mutex<Option<Waker>> }
    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let inner = Arc::new(OneInner { val: Mutex::new(None), waker: Mutex::new(None) });
        (Sender { inner: Some(inner.clone()) }, Receiver { inner })
    }
    impl<T> Sender<T> {
        pub fn send(self, val: T) -> Result<(), T> {
            if let Some(inner) = &self.inner {
                *inner.val.lock().unwrap() = Some(val);
                if let Some(w) = inner.waker.lock().unwrap().take() { w.wake(); }
                Ok(())
            } else { Err(val) }
        }
    }
    impl<T> Drop for Sender<T> {
        fn drop(&mut self) {
            if let Some(inner) = &self.inner {
                if let Some(w) = inner.waker.lock().unwrap().take() { w.wake(); }
            }
        }
    }
    impl<T> Future for Receiver<T> {
        type Output = Result<T, RecvError>;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if let Some(v) = self.inner.val.lock().unwrap().take() { Poll::Ready(Ok(v)) }
            else { *self.inner.waker.lock().unwrap() = Some(cx.waker().clone()); Poll::Pending }
        }
    }
    #[derive(Debug)] pub struct RecvError;
    impl std::fmt::Display for RecvError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("recv error") } }
}

// ===========================================================================
// Signal — ctrl_c
// ===========================================================================

static GOT_SIGINT: AtomicBool = AtomicBool::new(false);

pub fn ctrl_c() -> CtrlC {
    extern "C" fn handler(_: libc::c_int) { GOT_SIGINT.store(true, Ordering::Release); }
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| { unsafe { libc::signal(libc::SIGINT, handler as libc::sighandler_t); } });
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
