//! Async I/O utilities — `copy`, `copy_bidirectional`, `empty`, `sink`, `repeat`.
//!
//! These are thin wrappers that compose `AsyncRead` and `AsyncWrite`
//! for common data-flow patterns.

use std::future::Future;
use std::io::{self};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::io_traits::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

// ===========================================================================
// copy
// ===========================================================================

/// Copy all bytes from `reader` into `writer`.
///
/// Returns the number of bytes copied. Neither end is closed.
pub async fn copy<R, W>(reader: &mut R, writer: &mut W) -> io::Result<u64>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut total: u64 = 0;
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n]).await?;
        total += n as u64;
    }
    Ok(total)
}

// ===========================================================================
// copy_bidirectional
// ===========================================================================

/// Copy data bidirectionally between two streams.
///
/// Returns `(a_to_b, b_to_a)` — bytes copied in each direction.
/// Both streams must implement `AsyncRead` and `AsyncWrite`.
pub async fn copy_bidirectional<A, B>(a: &mut A, b: &mut B) -> io::Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf_a = [0u8; 8192];
    let mut buf_b = [0u8; 8192];
    let mut a_to_b: u64 = 0;
    let mut b_to_a: u64 = 0;

    // Alternate between reading from each side and writing to the other.
    // This is a simple round-robin approach — not the most efficient but correct.
    loop {
        // Read from A, write to B
        let n = a.read(&mut buf_a).await?;
        if n == 0 {
            break; // A closed
        }
        b.write_all(&buf_a[..n]).await?;
        a_to_b += n as u64;

        // Read from B, write to A
        let n = b.read(&mut buf_b).await?;
        if n == 0 {
            break; // B closed
        }
        a.write_all(&buf_b[..n]).await?;
        b_to_a += n as u64;
    }

    Ok((a_to_b, b_to_a))
}

// ===========================================================================
// empty / Empty
// ===========================================================================

/// Create an `AsyncRead` that immediately returns EOF.
pub fn empty() -> Empty {
    Empty { _p: () }
}

pub struct Empty {
    _p: (),
}

impl AsyncRead for Empty {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(0))
    }
}

// ===========================================================================
// sink / Sink
// ===========================================================================

/// Create an `AsyncWrite` that consumes and discards all data.
pub fn sink() -> Sink {
    Sink { _p: () }
}

pub struct Sink {
    _p: (),
}

impl AsyncWrite for Sink {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

// ===========================================================================
// repeat / Repeat
// ===========================================================================

/// Create an `AsyncRead` that repeatedly yields the given byte.
pub fn repeat(byte: u8) -> Repeat {
    Repeat { byte }
}

pub struct Repeat {
    byte: u8,
}

impl AsyncRead for Repeat {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        for b in buf.iter_mut() {
            *b = self.byte;
        }
        Poll::Ready(Ok(buf.len()))
    }
}
