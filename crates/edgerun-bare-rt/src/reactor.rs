//! I/O reactor stub for bare-metal runtime.


extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::Waker;

use crate::ready_queue::ReadyQueue;
use crate::sync_prim::{Condvar, Mutex};

pub struct FdInterest {
    state: Mutex<(Option<Waker>, Option<Waker>)>,
}

impl FdInterest {
    fn new() -> Self {
        Self {
            state: Mutex::new((None, None)),
        }
    }

    fn set_read_waker(&self, w: Waker) {
        self.state.lock().0 = Some(w);
    }

    fn set_write_waker(&self, w: Waker) {
        self.state.lock().1 = Some(w);
    }

    fn fire_read(&self) {
        if let Some(w) = self.state.lock().0.take() {
            w.wake();
        }
    }

    fn fire_write(&self) {
        if let Some(w) = self.state.lock().1.take() {
            w.wake();
        }
    }

    fn has_wakers(&self) -> bool {
        let s = self.state.lock();
        s.0.is_some() || s.1.is_some()
    }
}

pub struct Reactor {
    fds: Mutex<Vec<(isize, Arc<FdInterest>)>>,
    shutdown: AtomicBool,
}

impl Reactor {
    pub fn new() -> Self {
        Self {
            fds: Mutex::new(Vec::new()),
            shutdown: AtomicBool::new(false),
        }
    }

    pub fn get_or_register_fd(&self, fd: isize) -> Arc<FdInterest> {
        let mut fds = self.fds.lock();
        if let Some((_, interest)) = fds.iter().find(|(d, _)| *d == fd) {
            return interest.clone();
        }
        let interest = Arc::new(FdInterest::new());
        fds.push((fd, interest.clone()));
        interest
    }

    pub fn deregister_fd(&self, fd: isize) {
        self.fds.lock().retain(|(d, _)| *d != fd);
    }

    pub fn wait_read(&self, fd: isize, waker: Waker) {
        let interest = self.get_or_register_fd(fd);
        interest.set_read_waker(waker);
    }

    pub fn wait_write(&self, fd: isize, waker: Waker) {
        let interest = self.get_or_register_fd(fd);
        interest.set_write_waker(waker);
    }

    pub fn fire_fd(&self, fd: isize, read: bool, write: bool) {
        if let Some(interest) = self.fds.lock().iter().find(|(d, _)| *d == fd).map(|(_, i)| i) {
            if read {
                interest.fire_read();
            }
            if write {
                interest.fire_write();
            }
        }
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }
}