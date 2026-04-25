//! Reactor-based fd readiness futures (no_std stub).


extern crate alloc;

use alloc::sync::Arc;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::io_traits::Error;

pub struct FdReadReady {
    fd: isize,
    done: bool,
}

impl FdReadReady {
    pub fn new(fd: isize) -> Self {
        Self { fd, done: false }
    }
}

impl core::future::Future for FdReadReady {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.done {
            return Poll::Ready(Ok(()));
        }
        self.done = true;
        Poll::Pending
    }
}

pub struct FdWriteReady {
    fd: isize,
    done: bool,
}

impl FdWriteReady {
    pub fn new(fd: isize) -> Self {
        Self { fd, done: false }
    }
}

impl core::future::Future for FdWriteReady {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.done {
            return Poll::Ready(Ok(()));
        }
        self.done = true;
        Poll::Pending
    }
}