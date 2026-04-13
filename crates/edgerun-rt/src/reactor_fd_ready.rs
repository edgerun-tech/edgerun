//! Reactor-based fd readiness futures.
//!
//! `FdReadReady` and `FdWriteReady` are futures that resolve when the
//! given fd becomes readable or writable, respectively. They register
//! with the reactor's epoll and return `Poll::Ready(())` once the
//! reactor fires the waker.
//!
//! ## Correctness
//! On first poll, these futures check readiness via `poll(2)` with
//! timeout=0. If the fd is already ready, they return `Ready(())`
//! immediately. Otherwise, they register with the reactor and return
//! `Pending`, relying on the reactor to wake them when the fd becomes ready.

use std::future::Future;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::runtime::try_current_rt;

/// Future that resolves when the fd is readable.
pub(crate) struct FdReadReady {
    fd: RawFd,
    done: bool,
}

impl FdReadReady {
    pub(crate) fn new(fd: RawFd) -> Self {
        Self { fd, done: false }
    }
}

impl Future for FdReadReady {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.done {
            return Poll::Ready(());
        }
        // Check immediate readiness before registering with reactor.
        // If the fd is already readable, return Ready immediately
        // to avoid hanging on already-ready fds.
        let mut pfd = libc::pollfd {
            fd: self.fd,
            events: libc::POLLIN as _,
            revents: 0,
        };
        let res = unsafe { libc::poll(&mut pfd, 1, 0) };
        if res > 0 && (pfd.revents & libc::POLLIN as i16) != 0 {
            self.done = true;
            return Poll::Ready(());
        }

        if let Some(rt) = try_current_rt() {
            rt.reactor.wait_read(self.fd, cx.waker().clone());
        }
        self.done = true;
        Poll::Pending
    }
}

/// Future that resolves when the fd is writable.
pub(crate) struct FdWriteReady {
    fd: RawFd,
    done: bool,
}

impl FdWriteReady {
    pub(crate) fn new(fd: RawFd) -> Self {
        Self { fd, done: false }
    }
}

impl Future for FdWriteReady {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.done {
            return Poll::Ready(());
        }
        // Check immediate readiness before registering with reactor.
        // If the fd is already writable, return Ready immediately
        // to avoid hanging on already-ready fds.
        let mut pfd = libc::pollfd {
            fd: self.fd,
            events: libc::POLLOUT as _,
            revents: 0,
        };
        let res = unsafe { libc::poll(&mut pfd, 1, 0) };
        if res > 0 && (pfd.revents & libc::POLLOUT as i16) != 0 {
            self.done = true;
            return Poll::Ready(());
        }

        if let Some(rt) = try_current_rt() {
            rt.reactor.wait_write(self.fd, cx.waker().clone());
        }
        self.done = true;
        Poll::Pending
    }
}
