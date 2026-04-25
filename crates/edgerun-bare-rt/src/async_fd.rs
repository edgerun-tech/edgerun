//! AsyncFd — wrap any raw file descriptor with async readiness (no_std).

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use core::task::{Context, Poll};
use core::future::Future;

use crate::io_traits::Error;

pub struct AsyncFd<F> {
    fd: F,
    ready_read: AtomicBool,
    ready_write: AtomicBool,
}

impl<F: AsRawFd> AsyncFd<F> {
    pub fn new(fd: F) -> Result<Self, Error> {
        Ok(Self {
            fd,
            ready_read: AtomicBool::new(false),
            ready_write: AtomicBool::new(false),
        })
    }

    pub fn get_ref(&self) -> &F {
        &self.fd
    }

    pub fn get_mut(&mut self) -> &mut F {
        &mut self.fd
    }

    pub fn into_inner(self) -> F {
        self.fd
    }

    pub fn readable(&self) -> ReadyFuture {
        ReadyFuture {
            fd: self.fd.as_raw_fd(),
            dir: Direction::Read,
            ready_read: &self.ready_read,
            ready_write: &self.ready_write,
        }
    }

    pub fn writable(&self) -> ReadyFuture {
        ReadyFuture {
            fd: self.fd.as_raw_fd(),
            dir: Direction::Write,
            ready_read: &self.ready_read,
            ready_write: &self.ready_write,
        }
    }

    pub fn poll_read_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.poll_ready(true, cx)
    }

    pub fn poll_write_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.poll_ready(false, cx)
    }

    fn poll_ready(&self, read: bool, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let ready = if read {
            &self.ready_read
        } else {
            &self.ready_write
        };

        if ready.load(Ordering::Acquire) {
            ready.store(false, Ordering::Release);
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    pub fn try_io<R, C>(&self, f: C) -> Result<R, Error>
    where
        C: FnOnce() -> Result<R, Error>,
    {
        f()
    }

    pub fn try_io_mut<R, C>(&self, f: C) -> Result<R, Error>
    where
        C: FnOnce() -> Result<R, Error>,
    {
        f()
    }

    fn mark_ready(&self, read: bool) {
        if read {
            self.ready_read.store(true, Ordering::Release);
        } else {
            self.ready_write.store(true, Ordering::Release);
        }
    }
}

impl<F: AsRawFd + core::fmt::Debug> core::fmt::Debug for AsyncFd<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AsyncFd").field("inner", &self.fd).finish()
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Read,
    Write,
}

pub struct ReadyFuture<'a> {
    fd: isize,
    dir: Direction,
    ready_read: &'a AtomicBool,
    ready_write: &'a AtomicBool,
}

impl<'a> Future for ReadyFuture<'a> {
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ready = match self.dir {
            Direction::Read => self.ready_read,
            Direction::Write => self.ready_write,
        };

        if ready.load(Ordering::Acquire) {
            ready.store(false, Ordering::Release);
            Poll::Ready(Ok(()))
        } else {
            core::hint::spin_loop();
            Poll::Pending
        }
    }
}

pub struct OwnedAsyncFd {
    fd: isize,
}

impl OwnedAsyncFd {
    pub fn from_raw_fd(fd: isize) -> Self {
        Self { fd }
    }
}

impl AsRawFd for OwnedAsyncFd {
    fn as_raw_fd(&self) -> isize {
        self.fd
    }
}

pub fn async_fd_from_raw(fd: isize) -> Result<AsyncFd<OwnedAsyncFd>, Error> {
    AsyncFd::new(OwnedAsyncFd::from_raw_fd(fd))
}

pub fn pipe() -> Result<(AsyncFd<OwnedAsyncFd>, AsyncFd<OwnedAsyncFd>), Error> {
    Err(Error::new(crate::io_traits::ErrorKind::Other))
}

pub trait AsRawFd {
    fn as_raw_fd(&self) -> isize;
}

impl AsRawFd for isize {
    fn as_raw_fd(&self) -> isize {
        *self
    }
}