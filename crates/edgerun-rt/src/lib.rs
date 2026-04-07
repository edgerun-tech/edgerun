//! Minimal epoll-based async runtime for edgerun.

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
                Err(e) => { eprintln!("edgerun-rt: epoll: {}", e); std::thread::sleep(Duration::from_millis(10)); }
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers: no-op waker and context
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // ReadyQueue
    // -----------------------------------------------------------------------

    #[test]
    fn ready_queue_push_pop_ordering() {
        let q = ReadyQueue::new();
        q.push(10);
        q.push(20);
        q.push(30);
        assert_eq!(q.pop(), Some(10));
        assert_eq!(q.pop(), Some(20));
        assert_eq!(q.pop(), Some(30));
    }

    #[test]
    fn ready_queue_pop_empty_returns_none_after_shutdown() {
        let q = ReadyQueue::new();
        q.shutdown();
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn ready_queue_drain_then_shutdown() {
        let q = ReadyQueue::new();
        q.push(1);
        q.push(2);
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        q.shutdown();
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn ready_queue_shutdown_with_items_still_drains() {
        let q = ReadyQueue::new();
        q.push(42);
        q.push(99);
        q.shutdown();
        let mut got = Vec::new();
        while let Some(id) = q.pop() {
            got.push(id);
        }
        assert_eq!(got, vec![42, 99]);
    }

    // -----------------------------------------------------------------------
    // TaskMap
    // -----------------------------------------------------------------------

    #[test]
    fn task_map_insert_returns_id() {
        let tm = TaskMap::new();
        let id = tm.insert(Box::new(|_| false));
        assert!(id >= 1);
    }

    #[test]
    fn task_map_ids_are_unique_and_incrementing() {
        let tm = TaskMap::new();
        let id1 = tm.insert(Box::new(|_| false));
        let id2 = tm.insert(Box::new(|_| false));
        let id3 = tm.insert(Box::new(|_| false));
        assert!(id1 < id2 && id2 < id3);
    }

    #[test]
    fn task_map_take_returns_and_removes() {
        let tm = TaskMap::new();
        let id = tm.insert(Box::new(|_| false));
        assert!(tm.take(id).is_some());
        assert!(tm.take(id).is_none());
    }

    #[test]
    fn task_map_take_nonexistent() {
        let tm = TaskMap::new();
        assert!(tm.take(999).is_none());
    }

    #[test]
    fn task_map_first_id_is_one() {
        let tm = TaskMap::new();
        assert_eq!(tm.insert(Box::new(|_| false)), 1);
    }

    // -----------------------------------------------------------------------
    // Timer (heap ordering)
    // -----------------------------------------------------------------------

    #[test]
    fn timer_earliest_deadline_on_top() {
        let base = Instant::now();
        let mut heap = BinaryHeap::new();
        heap.push(Timer { deadline: base + Duration::from_secs(30), waker: NOOP_WAKER.clone() });
        heap.push(Timer { deadline: base + Duration::from_secs(5), waker: NOOP_WAKER.clone() });
        heap.push(Timer { deadline: base + Duration::from_secs(10), waker: NOOP_WAKER.clone() });
        assert_eq!(heap.peek().unwrap().deadline, base + Duration::from_secs(5));
    }

    #[test]
    fn timer_pop_ordering_earliest_first() {
        let base = Instant::now();
        let mut heap = BinaryHeap::new();
        heap.push(Timer { deadline: base + Duration::from_secs(3), waker: NOOP_WAKER.clone() });
        heap.push(Timer { deadline: base + Duration::from_secs(1), waker: NOOP_WAKER.clone() });
        heap.push(Timer { deadline: base + Duration::from_secs(2), waker: NOOP_WAKER.clone() });
        let d1 = heap.pop().unwrap().deadline;
        let d2 = heap.pop().unwrap().deadline;
        let d3 = heap.pop().unwrap().deadline;
        assert!(d1 <= d2 && d2 <= d3);
    }

    #[test]
    fn timer_cmp() {
        let base = Instant::now();
        let t1 = Timer { deadline: base + Duration::from_secs(5), waker: NOOP_WAKER.clone() };
        let t2 = Timer { deadline: base + Duration::from_secs(10), waker: NOOP_WAKER.clone() };
        assert!(t1 > t2);
    }

    #[test]
    fn timer_eq() {
        let base = Instant::now();
        let t1 = Timer { deadline: base + Duration::from_secs(5), waker: NOOP_WAKER.clone() };
        let t2 = Timer { deadline: base + Duration::from_secs(5), waker: NOOP_WAKER.clone() };
        assert!(t1 == t2);
    }

    // -----------------------------------------------------------------------
    // FdInterest
    // -----------------------------------------------------------------------

    #[test]
    fn fd_interest_starts_empty() {
        let fi = FdInterest::new();
        assert!(fi.read_waker.lock().unwrap().is_none());
        assert!(fi.write_waker.lock().unwrap().is_none());
    }

    // -----------------------------------------------------------------------
    // JoinError
    // -----------------------------------------------------------------------

    #[test]
    fn join_error_display() {
        assert_eq!(format!("{}", JoinError), "join error");
    }

    #[test]
    fn join_error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(JoinError);
        assert_eq!(format!("{}", e), "join error");
    }

    #[test]
    fn join_error_debug() {
        let s = format!("{:?}", JoinError);
        assert!(s.contains("JoinError"));
    }

    // -----------------------------------------------------------------------
    // JoinHandle
    // -----------------------------------------------------------------------

    #[test]
    fn join_handle_blocking_recv_ok() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Ok(42i32)).unwrap();
        let h: JoinHandle<i32> = JoinHandle { rx };
        assert_eq!(h.blocking_recv().unwrap(), 42);
    }

    #[test]
    fn join_handle_blocking_recv_err() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Err(JoinError)).unwrap();
        let h: JoinHandle<i32> = JoinHandle { rx };
        assert!(h.blocking_recv().is_err());
    }

    #[test]
    fn join_handle_future_poll_ready() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Ok(99i32)).unwrap();
        let mut h: JoinHandle<i32> = JoinHandle { rx };
        let p = Pin::new(&mut h);
        let result = p.poll(&mut ctx());
        assert!(matches!(result, Poll::Ready(Ok(99))));
    }

    #[test]
    fn join_handle_future_poll_pending() {
        let (_tx, rx) = std::sync::mpsc::channel::<Result<i32, JoinError>>();
        let mut h: JoinHandle<i32> = JoinHandle { rx };
        let p = Pin::new(&mut h);
        assert!(matches!(p.poll(&mut ctx()), Poll::Pending));
    }

    #[test]
    fn join_handle_future_poll_disconnected() {
        let rx = {
            let (tx, rx) = std::sync::mpsc::channel::<Result<i32, JoinError>>();
            drop(tx);
            rx
        };
        let mut h: JoinHandle<i32> = JoinHandle { rx };
        let p = Pin::new(&mut h);
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Err(JoinError))));
    }

    // -----------------------------------------------------------------------
    // ReadFut
    // -----------------------------------------------------------------------

    struct MockReader { result: Poll<io::Result<usize>> }
    impl AsyncRead for MockReader {
        fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
            match &self.result {
                Poll::Ready(Ok(n)) => {
                    let n = *n;
                    for b in buf.iter_mut().take(n) { *b = 0xAB; }
                    Poll::Ready(Ok(n))
                }
                Poll::Ready(Err(_)) => Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, "reset"))),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    #[test]
    fn readfut_ready_ok() {
        let mut r = MockReader { result: Poll::Ready(Ok(3)) };
        let mut buf = [0u8; 10];
        let mut f = ReadFut { s: &mut r, buf: &mut buf };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        let result = p.poll(&mut ctx());
        assert!(matches!(result, Poll::Ready(Ok(3))));
        assert!(buf[..3].iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn readfut_pending() {
        let mut r = MockReader { result: Poll::Pending };
        let mut buf = [0u8; 10];
        let mut f = ReadFut { s: &mut r, buf: &mut buf };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Pending));
    }

    #[test]
    fn readfut_error() {
        let mut r = MockReader { result: Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, "reset"))) };
        let mut buf = [0u8; 10];
        let mut f = ReadFut { s: &mut r, buf: &mut buf };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Err(_))));
    }

    // -----------------------------------------------------------------------
    // ReadExactFut
    // -----------------------------------------------------------------------

    struct MockReaderExact { calls: Arc<Mutex<usize>> }
    impl AsyncRead for MockReaderExact {
        fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
            let n = buf.len().min(3);
            *self.calls.lock().unwrap() += 1;
            Poll::Ready(Ok(n))
        }
    }

    #[test]
    fn read_exact_fut_completes() {
        let calls = Arc::new(Mutex::new(0usize));
        let mut r = MockReaderExact { calls: calls.clone() };
        let mut buf = [0u8; 6];
        let mut f = ReadExactFut { s: &mut r, buf: &mut buf, pos: 0 };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Ok(()))));
        assert_eq!(*calls.lock().unwrap(), 2);
    }

    #[test]
    fn read_exact_fut_already_done() {
        let calls = Arc::new(Mutex::new(0usize));
        let mut r = MockReaderExact { calls: calls.clone() };
        let mut buf = [0u8; 0];
        let mut f = ReadExactFut { s: &mut r, buf: &mut buf, pos: 0 };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Ok(()))));
        assert_eq!(*calls.lock().unwrap(), 0);
    }

    struct MockReaderEof;
    impl AsyncRead for MockReaderEof {
        fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &mut [u8]) -> Poll<io::Result<usize>> {
            Poll::Ready(Ok(0))
        }
    }

    #[test]
    fn read_exact_fut_eof() {
        let mut r = MockReaderEof;
        let mut buf = [0u8; 10];
        let mut f = ReadExactFut { s: &mut r, buf: &mut buf, pos: 0 };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Err(e)) if e.kind() == io::ErrorKind::UnexpectedEof));
    }

    // -----------------------------------------------------------------------
    // WriteAllFut
    // -----------------------------------------------------------------------

    struct MockWriter { calls: Arc<Mutex<usize>> }
    impl AsyncWrite for MockWriter {
        fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
            let n = buf.len().min(4);
            *self.calls.lock().unwrap() += 1;
            Poll::Ready(Ok(n))
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
    }

    #[test]
    fn write_all_fut_completes() {
        let calls = Arc::new(Mutex::new(0usize));
        let mut w = MockWriter { calls: calls.clone() };
        let data = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let mut f = WriteAllFut { s: &mut w, buf: &data, pos: 0 };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Ok(()))));
        assert_eq!(*calls.lock().unwrap(), 2);
    }

    #[test]
    fn write_all_fut_empty_buf() {
        let calls = Arc::new(Mutex::new(0usize));
        let mut w = MockWriter { calls: calls.clone() };
        let data: [u8; 0] = [];
        let mut f = WriteAllFut { s: &mut w, buf: &data, pos: 0 };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Ok(()))));
        assert_eq!(*calls.lock().unwrap(), 0);
    }

    #[test]
    fn write_all_fut_error() {
        struct FailingWriter;
        impl AsyncWrite for FailingWriter {
            fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &[u8]) -> Poll<io::Result<usize>> {
                Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken")))
            }
            fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
            fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
        }
        let mut w = FailingWriter;
        let mut f = WriteAllFut { s: &mut w, buf: &[1, 2, 3], pos: 0 };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Err(e)) if e.kind() == io::ErrorKind::BrokenPipe));
    }

    // -----------------------------------------------------------------------
    // FlushFut
    // -----------------------------------------------------------------------

    #[test]
    fn flush_fut_ready_ok() {
        let mut w = MockWriter { calls: Arc::new(Mutex::new(0usize)) };
        let mut f = FlushFut { s: &mut w };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Ok(()))));
    }

    struct FailingFlusher;
    impl AsyncWrite for FailingFlusher {
        fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &[u8]) -> Poll<io::Result<usize>> {
            Poll::Ready(Ok(0))
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "flush failed")))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
    }

    #[test]
    fn flush_fut_error() {
        let mut w = FailingFlusher;
        let mut f = FlushFut { s: &mut w };
        let p = unsafe { Pin::new_unchecked(&mut f) };
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Err(_))));
    }

    // -----------------------------------------------------------------------
    // Sleep
    // -----------------------------------------------------------------------

    #[test]
    fn sleep_past_deadline_is_ready() {
        let deadline = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        let mut s = Sleep { deadline };
        let p = Pin::new(&mut s);
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(())));
    }

    #[test]
    fn sleep_stores_deadline_correctly() {
        let future_deadline = Instant::now() + Duration::from_secs(10);
        let s = Sleep { deadline: future_deadline };
        assert!(s.deadline > Instant::now());
    }

    // -----------------------------------------------------------------------
    // Elapsed
    // -----------------------------------------------------------------------

    #[test]
    fn elapsed_display() {
        assert_eq!(format!("{}", Elapsed), "deadline elapsed");
    }

    #[test]
    fn elapsed_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(Elapsed);
        assert_eq!(format!("{}", e), "deadline elapsed");
    }

    #[test]
    fn elapsed_debug() {
        let s = format!("{:?}", Elapsed);
        assert!(s.contains("Elapsed"));
    }

    // -----------------------------------------------------------------------
    // Timeout
    // -----------------------------------------------------------------------

    #[test]
    fn timeout_struct_stores_fields() {
        let deadline = Instant::now() + Duration::from_secs(5);
        let t: Timeout<std::future::Ready<()>> = Timeout {
            inner: Some(std::future::ready(())),
            deadline,
        };
        assert!(t.deadline > Instant::now());
    }

    #[test]
    fn timeout_past_deadline_polls_elapsed() {
        let deadline = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        let mut t: Timeout<std::future::Pending<()>> = Timeout {
            inner: Some(std::future::pending()),
            deadline,
        };
        let p = Pin::new(&mut t);
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Err(Elapsed))));
    }

    // -----------------------------------------------------------------------
    // Interval
    // -----------------------------------------------------------------------

    #[test]
    fn interval_initial_next_is_approx_now_plus_duration() {
        let d = Duration::from_millis(100);
        let before = Instant::now();
        let iv = interval(d);
        let after = Instant::now();
        let expected_min = before + d;
        let expected_max = after + d + Duration::from_millis(50);
        assert!(iv.next >= expected_min);
        assert!(iv.next <= expected_max);
    }

    #[test]
    fn interval_tick_past_deadline_is_ready() {
        let mut iv = Interval {
            d: Duration::from_millis(50),
            next: Instant::now().checked_sub(Duration::from_millis(10)).unwrap(),
            _mb: MissedTickBehavior::Skip,
        };
        let mut t = iv.tick();
        let p = Pin::new(&mut t);
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(())));
        // next advanced by d
        assert!(iv.next > Instant::now().checked_sub(Duration::from_millis(100)).unwrap());
    }

    #[test]
    fn interval_set_missed_tick_behavior() {
        let mut iv = interval(Duration::from_millis(100));
        assert_eq!(iv._mb, MissedTickBehavior::Skip);
        iv.set_missed_tick_behavior(MissedTickBehavior::Skip);
        assert_eq!(iv._mb, MissedTickBehavior::Skip);
    }

    #[test]
    fn missed_tick_behavior_debug() {
        assert!(format!("{:?}", MissedTickBehavior::Skip).contains("Skip"));
    }

    // -----------------------------------------------------------------------
    // mpsc channel
    // -----------------------------------------------------------------------

    #[test]
    fn mpsc_send_error_display() {
        assert_eq!(format!("{}", mpsc::SendError("test")), "send error");
    }

    #[test]
    fn mpsc_channel_constructs() {
        let (tx, rx) = mpsc::channel::<i32>(16);
        drop((tx, rx));
    }

    #[test]
    fn mpsc_sender_clone() {
        let (tx1, _rx) = mpsc::channel::<i32>(16);
        let tx2 = tx1.clone();
        drop((tx1, tx2));
    }

    // -----------------------------------------------------------------------
    // oneshot channel
    // -----------------------------------------------------------------------

    #[test]
    fn oneshot_recv_error_display() {
        assert_eq!(format!("{}", oneshot::RecvError), "recv error");
    }

    #[test]
    fn oneshot_receiver_poll_pending_then_ready() {
        let (tx, mut rx) = oneshot::channel::<i32>();
        let p = Pin::new(&mut rx);
        assert!(matches!(p.poll(&mut ctx()), Poll::Pending));
        tx.send(42).unwrap();
        let p = Pin::new(&mut rx);
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Ok(42))));
    }

    #[test]
    fn oneshot_send_consumes_sender() {
        let (tx, _rx) = oneshot::channel::<i32>();
        tx.send(99).unwrap();
    }

    #[test]
    fn oneshot_channel_constructs() {
        let (tx, rx) = oneshot::channel::<String>();
        drop((tx, rx));
    }

    // -----------------------------------------------------------------------
    // Builder
    // -----------------------------------------------------------------------

    #[test]
    fn builder_new_multi_thread_defaults() {
        let b = Builder::new_multi_thread();
        assert!(b.workers >= 1);
    }

    #[test]
    fn builder_enable_all_returns_self() {
        let mut b = Builder::new_multi_thread();
        let ret = b.enable_all();
        assert!(std::ptr::eq(ret, &b));
    }

    // -----------------------------------------------------------------------
    // TcpListener / TcpStream futures
    // -----------------------------------------------------------------------

    #[test]
    fn bind_future_struct_constructs() {
        let _bf: BindFuture<std::net::SocketAddr> = BindFuture {
            addr: Some("127.0.0.1:0".parse().unwrap()),
            _marker: std::marker::PhantomData,
        };
    }

    #[test]
    fn connect_future_struct_constructs() {
        let _cf = ConnectFuture { addrs: None, done: false };
    }

    #[test]
    fn connect_future_with_no_addrs_returns_error() {
        let mut cf = ConnectFuture { addrs: None, done: false };
        let p = Pin::new(&mut cf);
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Err(e)) if e.kind() == io::ErrorKind::InvalidInput));
    }

    #[test]
    fn connect_future_done_flag_returns_error() {
        let mut cf = ConnectFuture {
            addrs: Some(vec!["127.0.0.1:1".parse().unwrap()]),
            done: true,
        };
        let p = Pin::new(&mut cf);
        assert!(matches!(p.poll(&mut ctx()), Poll::Ready(Err(e)) if e.kind() == io::ErrorKind::Other));
    }
}
