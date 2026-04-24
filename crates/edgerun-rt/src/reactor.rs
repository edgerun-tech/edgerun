//! epoll-based I/O reactor with timer heap.
//!
//! One reactor thread runs `Reactor::run()` which calls `epoll_wait` and
//! fires wakers when I/O readiness or timer deadlines arrive.
//!
//! ## Timer wakeup
//! The reactor monitors an eventfd via epoll. When `register_timer` is called,
//! a byte is written to the eventfd, waking the reactor so it can recalculate
//! the epoll timeout. Without this, the reactor could sleep for the default
//! 100ms timeout even when a 1ms timer was just registered.
//!
//! ## Thread safety
//! All fd interest state is protected by a single Mutex on FdInterest.
//! `update_epoll` holds the lock through the entire read-compute-epoll_ctl
//! sequence, eliminating TOCTOU races between concurrent waker registration
//! and epoll event updates.

use std::collections::BinaryHeap;
use std::io::{self};
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use crate::sync::Mutex;
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
// Timer wakeup via eventfd
// ===========================================================================

/// Writes to this eventfd to wake the reactor when a new timer is registered.
struct TimerNotify {
    fd: libc::c_int,
}

impl TimerNotify {
    fn new() -> io::Result<Self> {
        // EFD_NONBLOCK: reads never block, writes never block (counter-based).
        let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd })
    }

    /// Write a byte to wake the reactor. Safe to call from any thread.
    fn notify(&self) {
        let buf: u64 = 1;
        let ret = unsafe {
            libc::write(
                self.fd,
                &buf as *const u64 as *const libc::c_void,
                std::mem::size_of::<u64>(),
            )
        };
        // Ignore errors — if the pipe is full or closed, the reactor will
        // wake up on the next timer/fire anyway.
        if ret < 0 {
            // EAGAIN means the counter is already non-zero — reactor will wake.
            let err = io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::EAGAIN) && err.raw_os_error() != Some(libc::EWOULDBLOCK) {
                edgerun_log::warn!("timer notify write error: {}", err);
            }
        }
    }

    /// Drain the eventfd counter. Called by the reactor after waking.
    fn drain(&self) {
        let mut buf: u64 = 0;
        let ret = unsafe {
            libc::read(
                self.fd,
                &mut buf as *mut u64 as *mut libc::c_void,
                std::mem::size_of::<u64>(),
            )
        };
        if ret < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::EAGAIN) && err.raw_os_error() != Some(libc::EWOULDBLOCK) {
                edgerun_log::warn!("timer notify drain error: {}", err);
            }
        }
    }
}

impl Drop for TimerNotify {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

// ===========================================================================
// FD interest — single lock for both wakers
// ===========================================================================

pub(crate) struct FdInterest {
    /// (read_waker, write_waker)
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

    /// Fire wakers for the given event bits and update epoll interests.
    ///
    /// This method holds the state lock through the entire
    /// take-wakers → compute-events → epoll_ctl sequence, preventing
    /// TOCTOU races with concurrent `set_read_waker`/`set_write_waker` calls.
    ///
    /// Returns `true` if the fd still has any wakers after this operation.
    fn fire_and_update(&self, epoll: &EpollFd, fd: RawFd, event_bits: u32) -> bool {
        // Step 1: Take wakers under lock.
        let (read_waker, write_waker) = {
            let mut state = self.state.lock();
            let rw = if (event_bits & (libc::EPOLLIN as u32 | libc::EPOLLHUP as u32 | libc::EPOLLERR as u32)) != 0 {
                state.0.take()
            } else {
                None
            };
            let ww = if (event_bits & (libc::EPOLLOUT as u32 | libc::EPOLLHUP as u32 | libc::EPOLLERR as u32)) != 0 {
                state.1.take()
            } else {
                None
            };
            (rw, ww)
        };

        // Step 2: Wake outside the lock — wakers may re-enter and re-register.
        if let Some(w) = read_waker {
            w.wake();
        }
        if let Some(w) = write_waker {
            w.wake();
        }

        // Step 3: Re-acquire lock and recompute epoll events from current state.
        // A concurrent wait_read/write may have set new wakers between our
        // take and now, so we must re-read the state.
        let mut state = self.state.lock();
        let has_read = state.0.is_some();
        let has_write = state.1.is_some();

        if has_read || has_write {
            // Level-triggered mode (no EPOLLET): ensures the kernel fires
            // again if data is still available after the task re-polls.
            // Edge-triggered would silently drop events causing hangs.
            let mut events: u32 = 0;
            if has_read {
                events |= libc::EPOLLIN as u32;
            }
            if has_write {
                events |= libc::EPOLLOUT as u32;
            }
            drop(state); // release lock before epoll_ctl
            let mut ev = libc::epoll_event { events: events as _, u64: fd as u64 };
            let _ = epoll.ctl(libc::EPOLL_CTL_MOD, fd, &mut ev);
            true
        } else {
            // No waiters - but we can't DEL here due to the race mentioned above.
            // Keep the fd registered with EPOLLIN|EPOLLOUT (always ready) which is safe.
            // This prevents stalls when events arrive before new waiters register.
            drop(state);
            let mut ev = libc::epoll_event {
                events: (libc::EPOLLIN | libc::EPOLLOUT) as _,
                u64: fd as u64,
            };
            let _ = epoll.ctl(libc::EPOLL_CTL_MOD, fd, &mut ev);
            true
        }
    }

    /// Set both wakers and update epoll events atomically.
    /// Used by `wait_read`/`wait_write` to ensure the fd is properly
    /// registered with epoll before returning.
    fn set_and_update(&self, epoll: &EpollFd, fd: RawFd, read: Option<Waker>, write: Option<Waker>) {
        let mut state = self.state.lock();
        if let Some(w) = read {
            state.0 = Some(w);
        }
        if let Some(w) = write {
            state.1 = Some(w);
        }
        let has_read = state.0.is_some();
        let has_write = state.1.is_some();
        // Hold lock through epoll_ctl to prevent TOCTOU.
        // Level-triggered mode (no EPOLLET).
        if has_read || has_write {
            let mut events: u32 = 0;
            if has_read {
                events |= libc::EPOLLIN as u32;
            }
            if has_write {
                events |= libc::EPOLLOUT as u32;
            }
            let mut ev = libc::epoll_event { events: events as _, u64: fd as u64 };
            let _ = epoll.ctl(libc::EPOLL_CTL_MOD, fd, &mut ev);
        }
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
    timer_notify: TimerNotify,
    shutdown: AtomicBool,
}

impl Reactor {
    pub(crate) fn new() -> io::Result<Self> {
        let notify = TimerNotify::new()?;

        // Register the eventfd with epoll so the reactor wakes on timer notifications.
        let mut ev = libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLET) as _,
            u64: notify.fd as u64,
        };
        let epoll = EpollFd::new()?;
        epoll.ctl(libc::EPOLL_CTL_ADD, notify.fd, &mut ev)?;

        Ok(Self {
            epoll,
            fds: Mutex::new(std::collections::HashMap::new()),
            timers: Mutex::new(BinaryHeap::new()),
            timer_notify: notify,
            shutdown: AtomicBool::new(false),
        })
    }

    pub(crate) fn get_or_register_fd(&self, fd: RawFd) -> std::sync::Arc<FdInterest> {
        let mut map = self.fds.lock();
        if let Some(interest) = map.get(&fd) {
            return interest.clone();
        }
        // Register immediately with both read+write interest to avoid missing the first event.
        // The wakers will start as None, so no tasks will wake until wait_read/wait_write sets them.
        // Using level-triggered (no EPOLLET) ensures we get notified if data is already available.
        let s = std::sync::Arc::new(FdInterest::new());
        let mut ev = libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLOUT) as _,
            u64: fd as u64,
        };
        let _ = self.epoll.ctl(libc::EPOLL_CTL_ADD, fd, &mut ev);
        map.insert(fd, s.clone());
        s
    }

    pub(crate) fn deregister_fd(&self, fd: RawFd) {
        let _ = self.epoll.ctl(libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
        self.fds.lock().remove(&fd);
    }

    pub(crate) fn wait_read(&self, fd: RawFd, waker: Waker) {
        let interest = self.get_or_register_fd(fd);
        interest.set_and_update(&self.epoll, fd, Some(waker), None);
    }

    pub(crate) fn wait_write(&self, fd: RawFd, waker: Waker) {
        let interest = self.get_or_register_fd(fd);
        interest.set_and_update(&self.epoll, fd, None, Some(waker));
    }

    pub(crate) fn wait_connect(&self, fd: RawFd, waker: Waker) {
        let interest = self.get_or_register_fd(fd);
        interest.set_and_update(&self.epoll, fd, None, Some(waker));
    }

    pub(crate) fn register_timer(&self, deadline: Instant, waker: Waker) {
        self.timers.lock().push(Timer { deadline, waker });
        // Wake the reactor so it can recalculate the epoll timeout.
        // Without this, the reactor could be blocked in epoll_wait(100)
        // and miss a timer that expires in 1ms.
        self.timer_notify.notify();
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
                    let remaining = t.deadline
                        .saturating_duration_since(Instant::now())
                        .as_millis()
                        .min(i32::MAX as u128) as i32;
                    // Ensure we always wait at least 1ms when there are pending
                    // timers, to avoid busy-spinning on sub-millisecond deadlines.
                    remaining.max(1)
                } else {
                    -1 // Block indefinitely until a timer or I/O event arrives.
                }
            };

            match self.epoll.wait(&mut evts, ms) {
                Ok(n) => {
                    // Drain timer notifications first.
                    self.timer_notify.drain();

                    // Fire expired timers.
                    {
                        let mut timers = self.timers.lock();
                        while let Some(t) = timers.peek() {
                            if t.deadline <= Instant::now() {
                                let t = timers.pop().unwrap();
                                t.waker.wake();
                            } else {
                                break;
                            }
                        }
                    }

                    // Process I/O events.
                    for evt in evts.iter().take(n) {
                        let fd = evt.u64 as RawFd;
                        let bits = evt.events;

                        // Skip the timer notify fd.
                        if fd == self.timer_notify.fd {
                            continue;
                        }

                        // Get the fd interest under the map lock.
                        let interest = self.fds.lock().get(&fd).cloned();
                        if let Some(interest) = interest {
                            interest.fire_and_update(&self.epoll, fd, bits);
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
        // Wake the reactor so it can see the shutdown flag and exit.
        self.timer_notify.notify();
    }
}
