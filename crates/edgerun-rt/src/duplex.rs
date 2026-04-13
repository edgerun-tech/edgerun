//! DuplexStream — two connected in-memory async streams.
//!
//! Creates a pair of `AsyncRead + AsyncWrite` endpoints connected
//! back-to-back, similar to `tokio::io::duplex`. Useful for testing,
//! mock connections, and in-process communication.

use std::collections::VecDeque;
use std::io::{self};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use crate::sync::Mutex;

use crate::io_traits::{AsyncRead, AsyncWrite};

struct DuplexInner {
    buf: VecDeque<u8>,
    closed: bool,
    waker: Option<Waker>,
}

struct DuplexShared {
    a_to_b: Mutex<DuplexInner>,
    b_to_a: Mutex<DuplexInner>,
}

/// One half of a duplex in-memory stream pair.
pub struct DuplexStream {
    shared: Arc<DuplexShared>,
    /// True if this is the "B" side (reads from a_to_b, writes to b_to_a).
    is_b: bool,
}

impl DuplexStream {
    /// Create a pair of connected duplex streams.
    ///
    /// Data written to `a` can be read from `b`, and vice versa.
    pub fn channel() -> (Self, Self) {
        let shared = Arc::new(DuplexShared {
            a_to_b: Mutex::new(DuplexInner {
                buf: VecDeque::new(),
                closed: false,
                waker: None,
            }),
            b_to_a: Mutex::new(DuplexInner {
                buf: VecDeque::new(),
                closed: false,
                waker: None,
            }),
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

    fn read_inner(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut inner = if self.is_b {
            self.shared.a_to_b.lock()
        } else {
            self.shared.b_to_a.lock()
        };

        if inner.buf.is_empty() {
            if inner.closed {
                Ok(0)
            } else {
                Err(io::Error::new(io::ErrorKind::WouldBlock, "no data available"))
            }
        } else {
            let n = std::cmp::min(buf.len(), inner.buf.len());
            for (i, b) in inner.buf.drain(..n).enumerate() {
                buf[i] = b;
            }
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

        inner.buf.extend(buf.iter().copied());
        let n = buf.len();
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
        n
    }

    fn poll_read_ready(&self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut inner = if self.is_b {
            self.shared.a_to_b.lock()
        } else {
            self.shared.b_to_a.lock()
        };

        if !inner.buf.is_empty() || inner.closed {
            Poll::Ready(Ok(()))
        } else {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }

    /// Returns `true` if the write side has been shut down.
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
    ) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        match this.read_inner(buf) {
            Ok(0) => Poll::Ready(Ok(0)),
            Ok(n) => Poll::Ready(Ok(n)),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                match this.poll_read_ready(cx) {
                    Poll::Ready(Ok(())) => Poll::Ready(Ok(0)),
                    Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                    Poll::Pending => Poll::Pending,
                }
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl AsyncWrite for DuplexStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // Writes always succeed (in-memory buffer).
        let n = unsafe { self.get_unchecked_mut() }.write_inner(buf);
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut inner = if this.is_b {
            this.shared.b_to_a.lock()
        } else {
            this.shared.a_to_b.lock()
        };
        inner.closed = true;
        // Wake any reader waiting on this side.
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
        Poll::Ready(Ok(()))
    }
}

impl Unpin for DuplexStream {}
