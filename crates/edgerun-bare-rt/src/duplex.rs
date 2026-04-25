//! DuplexStream — two connected in-memory async streams (no_std).

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use crate::io_traits::{AsyncRead, AsyncWrite, Error};
use crate::sync_prim::Mutex;

struct DuplexInner {
    buf: Vec<u8>,
    read_pos: usize,
    closed: bool,
    wakers: Vec<Waker>,
}

impl DuplexInner {
    fn new() -> Self {
        Self {
            buf: Vec::new(),
            read_pos: 0,
            closed: false,
            wakers: Vec::new(),
        }
    }
}

struct DuplexShared {
    a_to_b: Mutex<DuplexInner>,
    b_to_a: Mutex<DuplexInner>,
}

pub struct DuplexStream {
    shared: Arc<DuplexShared>,
    is_b: bool,
}

impl DuplexStream {
    pub fn channel() -> (Self, Self) {
        let shared = Arc::new(DuplexShared {
            a_to_b: Mutex::new(DuplexInner::new()),
            b_to_a: Mutex::new(DuplexInner::new()),
        });
        let a = DuplexStream {
            shared: shared.clone(),
            is_b: false,
        };
        let b = DuplexStream {
            shared: shared.clone(),
            is_b: true,
        };
        (a, b)
    }

    fn read_inner(&self, buf: &mut [u8]) -> Result<usize, Error> {
        let mut inner = if self.is_b {
            self.shared.a_to_b.lock()
        } else {
            self.shared.b_to_a.lock()
        };

        if inner.read_pos >= inner.buf.len() {
            if inner.closed {
                Ok(0)
            } else {
                Err(Error::new(crate::io_traits::ErrorKind::WouldBlock))
            }
        } else {
            let available = &inner.buf[inner.read_pos..];
            let n = core::cmp::min(buf.len(), available.len());
            buf[..n].copy_from_slice(&available[..n]);
            inner.read_pos += n;
            Ok(n)
        }
    }

    fn write_inner(&self, buf: &[u8]) -> usize {
        let mut inner = if self.is_b {
            self.shared.b_to_a.lock()
        } else {
            self.shared.a_to_b.lock()
        };

        if inner.closed {
            return 0;
        }

        inner.buf.extend_from_slice(buf);
        let n = buf.len();
        let wakers = core::mem::take(&mut inner.wakers);
        drop(inner);
        for w in wakers {
            w.wake();
        }
        n
    }

    fn poll_read_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let mut inner = if self.is_b {
            self.shared.a_to_b.lock()
        } else {
            self.shared.b_to_a.lock()
        };

        if inner.read_pos < inner.buf.len() || inner.closed {
            Poll::Ready(Ok(()))
        } else {
            inner.wakers.push(cx.waker().clone());
            Poll::Pending
        }
    }

    pub fn is_closed(&self) -> bool {
        let inner = if self.is_b {
            self.shared.b_to_a.lock()
        } else {
            self.shared.a_to_b.lock()
        };
        inner.closed
    }
}

impl Clone for DuplexStream {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            is_b: self.is_b,
        }
    }
}

impl AsyncRead for DuplexStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        let this = unsafe { self.get_unchecked_mut() };
        loop {
            match this.read_inner(buf) {
                Ok(0) => return Poll::Ready(Ok(0)),
                Ok(n) => return Poll::Ready(Ok(n)),
                Err(ref e) if e.kind() == crate::io_traits::ErrorKind::WouldBlock => {
                    match this.poll_read_ready(cx) {
                        Poll::Ready(Ok(())) => continue,
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }
}

impl AsyncWrite for DuplexStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let n = unsafe { self.get_unchecked_mut() }.write_inner(buf);
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut inner = if this.is_b {
            this.shared.b_to_a.lock()
        } else {
            this.shared.a_to_b.lock()
        };
        inner.closed = true;
        let wakers = core::mem::take(&mut inner.wakers);
        drop(inner);
        for w in wakers {
            w.wake();
        }
        Poll::Ready(Ok(()))
    }
}

impl Unpin for DuplexStream {}