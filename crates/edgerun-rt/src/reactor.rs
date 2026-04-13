//! epoll-based I/O reactor with timer heap.
//!
//! One reactor thread runs `Reactor::run()` which calls `epoll_wait` and
//! fires wakers when I/O readiness or timer deadlines arrive.

use std::collections::BinaryHeap;
use std::io::{self};
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use parking_lot::Mutex;
use std::task::Waker;

use crate::ready_queue::ReadyQueue;
pub use std::time::Instant;

// ===========================================================================
// epoll wrapper
// ===========================================================================

struct EpollFd(libc::c_int);

impl EpollFd {
    fn new() -> io::Result<Self> {
        let fd = unsafe { libc::epoll_create1(0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(fd))
    }

    fn ctl(&self, op: libc::c_int, fd: RawFd, ev: *mut libc::epoll_event) -> io::Result<()> {
        if unsafe { libc::epoll_ctl(self.0, op, fd, ev) } < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn wait(&self, out: &mut [libc::epoll_event], ms: i32) -> io::Result<usize> {
        let n = unsafe { libc::epoll_wait(self.0, out.as_mut_ptr(), out.len() as _, ms) };
        if n < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(n as usize)
        }
    }
}

impl Drop for EpollFd {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}

// ===========================================================================
// FD interest — single lock for both wakers
// ===========================================================================

pub(crate) struct FdInterest {
    state: Mutex<(Option<Waker>, Option<Waker>)>,
}

impl FdInterest {
    fn new() -> Self {
        Self { state: Mutex::new((None, None)) }
    }

    fn take_read_waker(&self) -> Option<Waker> {
        self.state.lock().0.take()
    }

    fn take_write_waker(&self) -> Option<Waker> {
        self.state.lock().1.take()
    }

    fn set_read_waker(&self, w: Waker) {
        self.state.lock().0 = Some(w);
    }

    fn set_write_waker(&self, w: Waker) {
        self.state.lock().1 = Some(w);
    }

    fn has_read_waker(&self) -> bool {
        self.state.lock().0.is_some()
    }

    fn has_write_waker(&self) -> bool {
        self.state.lock().1.is_some()
    }

    fn update_epoll(&self, epoll: &EpollFd, fd: RawFd) {
        let state = self.state.lock();
        let has_read = state.0.is_some();
        let has_write = state.1.is_some();
        drop(state);
        let mut events: u32 = 0;
        if has_read {
            events |= libc::EPOLLIN as u32 | libc::EPOLLET as u32;
        }
        if has_write {
            events |= libc::EPOLLOUT as u32 | libc::EPOLLET as u32;
        }
        if events != 0 {
            let mut ev = libc::epoll_event { events: events as _, u64: fd as u64 };
            let _ = epoll.ctl(libc::EPOLL_CTL_MOD, fd, &mut ev);
        } else {
            let _ = epoll.ctl(libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
        }
    }

    fn has_any(&self) -> bool {
        let state = self.state.lock();
        state.0.is_some() || state.1.is_some()
    }
}

// ===========================================================================
// Timer heap
// ===========================================================================

struct Timer {
    deadline: Instant,
    waker: Waker,
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.deadline.cmp(&self.deadline)
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}

impl Eq for Timer {}

// ===========================================================================
// Reactor
// ===========================================================================

pub(crate) struct Reactor {
    epoll: EpollFd,
    fds: Mutex<std::collections::HashMap<RawFd, std::sync::Arc<FdInterest>>>,
    timers: Mutex<BinaryHeap<Timer>>,
    shutdown: AtomicBool,
}

impl Reactor {
    pub(crate) fn new() -> io::Result<Self> {
        Ok(Self {
            epoll: EpollFd::new()?,
            fds: Mutex::new(std::collections::HashMap::new()),
            timers: Mutex::new(BinaryHeap::new()),
            shutdown: AtomicBool::new(false),
        })
    }

    pub(crate) fn get_or_register_fd(&self, fd: RawFd) -> std::sync::Arc<FdInterest> {
        let mut map = self.fds.lock();
        map.entry(fd)
            .or_insert_with(|| {
                let s = std::sync::Arc::new(FdInterest::new());
                let mut ev = libc::epoll_event { events: 0, u64: fd as u64 };
                let _ = self.epoll.ctl(libc::EPOLL_CTL_ADD, fd, &mut ev);
                s
            })
            .clone()
    }

    pub(crate) fn deregister_fd(&self, fd: RawFd) {
        let _ = self.epoll.ctl(libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
        self.fds.lock().remove(&fd);
    }

    pub(crate) fn wait_read(&self, fd: RawFd, waker: Waker) {
        let interest = self.get_or_register_fd(fd);
        interest.set_read_waker(waker);
        interest.update_epoll(&self.epoll, fd);
    }

    pub(crate) fn wait_write(&self, fd: RawFd, waker: Waker) {
        let interest = self.get_or_register_fd(fd);
        interest.set_write_waker(waker);
        interest.update_epoll(&self.epoll, fd);
    }

    pub(crate) fn wait_connect(&self, fd: RawFd, waker: Waker) {
        let interest = self.get_or_register_fd(fd);
        interest.set_write_waker(waker);
        interest.update_epoll(&self.epoll, fd);
    }

    pub(crate) fn register_timer(&self, deadline: Instant, waker: Waker) {
        self.timers.lock().push(Timer { deadline, waker });
    }

    pub(crate) fn run(&self, _queue: &std::sync::Arc<ReadyQueue>) {
        const MAX_EVENTS: usize = 1024;
        let mut evts: [libc::epoll_event; MAX_EVENTS] =
            [libc::epoll_event { events: 0, u64: 0 }; MAX_EVENTS];

        loop {
            if self.shutdown.load(Ordering::Acquire) {
                break;
            }

            let ms = {
                let timers = self.timers.lock();
                if let Some(t) = timers.peek() {
                    t.deadline
                        .saturating_duration_since(Instant::now())
                        .as_millis()
                        .min(i32::MAX as u128) as i32
                } else {
                    100
                }
            };

            match self.epoll.wait(&mut evts, ms) {
                Ok(n) => {
                    {
                        let mut timers = self.timers.lock();
                        let now = Instant::now();
                        while let Some(t) = timers.peek() {
                            if t.deadline <= now {
                                let t = timers.pop().unwrap();
                                t.waker.wake();
                            } else {
                                break;
                            }
                        }
                    }

                    for evt in evts.iter().take(n) {
                        let fd = evt.u64 as RawFd;
                        let bits = evt.events;

                        if let Some(interest) = self.fds.lock().get(&fd) {
                            if (bits & (libc::EPOLLIN as u32 | libc::EPOLLHUP as u32 | libc::EPOLLERR as u32)) != 0 {
                                if let Some(w) = interest.take_read_waker() {
                                    w.wake();
                                }
                            }
                            if (bits & (libc::EPOLLOUT as u32 | libc::EPOLLHUP as u32 | libc::EPOLLERR as u32)) != 0 {
                                if let Some(w) = interest.take_write_waker() {
                                    w.wake();
                                }
                            }
                            interest.update_epoll(&self.epoll, fd);

                            if !interest.has_any() {
                                self.deregister_fd(fd);
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    edgerun_log::warn!("epoll error: {}", e);
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    pub(crate) fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}
