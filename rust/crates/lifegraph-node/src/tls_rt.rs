//! TLS integration for lifegraph-rt — wraps rustls with async I/O.

use std::future::Future;
use std::io::{Read, Write};
use std::io::{self, ErrorKind};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use lifegraph_rt::{AsyncRead, AsyncWrite, TcpStream};

/// TLS stream wrapping a TcpStream with a rustls ServerConnection.
pub struct TlsStream {
    stream: TcpStream,
    conn: rustls::ServerConnection,
}

impl TlsStream {
    fn new(stream: TcpStream, conn: rustls::ServerConnection) -> Self {
        Self { stream, conn }
    }

    /// Drive the TLS handshake. Returns Ready(Ok(())) when handshake is complete.
    fn drive_handshake(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            if !self.conn.is_handshaking() {
                return Poll::Ready(Ok(()));
            }

            // Write pending TLS records
            if self.poll_write_tls(cx)? == Poll::Ready(false) {
                // Underlying stream said WouldBlock — nothing more we can do
            }

            if !self.conn.is_handshaking() {
                return Poll::Ready(Ok(()));
            }

            // Read TLS records from underlying stream
            match self.poll_read_tls(cx)? {
                Poll::Ready(0) => return Poll::Ready(Err(io::Error::new(ErrorKind::UnexpectedEof, "tls eof"))),
                Poll::Ready(_) => {
                    // Process any received records
                    match self.conn.process_new_packets() {
                        Ok(_) => {}
                        Err(e) => return Poll::Ready(Err(io::Error::new(ErrorKind::InvalidData, e.to_string()))),
                    }
                }
                Poll::Pending => {}
            }

            // Check progress
            if self.conn.wants_read() && !self.conn.wants_write() {
                return Poll::Pending; // waiting for more data from peer
            }
            if self.conn.wants_write() && !self.conn.wants_read() {
                // We wrote something, loop again to check handshake status
                continue;
            }
            if self.conn.wants_read() && self.conn.wants_write() {
                // Both — wrote what we could, waiting for peer
                return Poll::Pending;
            }
            // Neither wants read or write but still handshaking? Shouldn't happen.
            return Poll::Pending;
        }
    }

    /// Write TLS records to the underlying stream.
    /// Returns Poll::Ready(true) if data was written, Poll::Ready(false) if WouldBlock.
    fn poll_write_tls(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<bool>> {
        let mut wrote = false;
        while self.conn.wants_write() {
            match self.conn.write_tls(&mut Writer(&mut self.stream)) {
                Ok(0) => break,
                Ok(n) => { wrote = true; continue; }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // The underlying stream needs write readiness
                    // Write returns WouldBlock — we need to poll the stream
                    match Pin::new(&mut self.stream).poll_write(cx, &[]) {
                        Poll::Ready(Ok(_)) => continue, // shouldn't happen with empty buf
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Pending => return Poll::Ready(Ok(false)),
                    }
                }
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
        Poll::Ready(Ok(wrote))
    }

    /// Read TLS records from the underlying stream.
    /// Returns Poll::Ready(bytes_read) or Poll::Ready(0) for EOF.
    fn poll_read_tls(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        match self.conn.read_tls(&mut Reader(&mut self.stream)) {
            Ok(0) => Poll::Ready(Ok(0)),
            Ok(n) => Poll::Ready(Ok(n)),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // Underlying stream needs read readiness
                let mut buf = [0u8; 4096];
                match Pin::new(&mut self.stream).poll_read(cx, &mut buf) {
                    Poll::Ready(Ok(0)) => Poll::Ready(Ok(0)),
                    Poll::Ready(Ok(n)) => {
                        // Feed data into rustls
                        self.conn.read_tls(&mut std::io::Cursor::new(&buf[..n]))?;
                        Poll::Ready(Ok(n))
                    }
                    Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                    Poll::Pending => Poll::Pending,
                }
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

/// Sync reader adapter — always returns WouldBlock so the caller polls the async stream.
struct Reader<'a>(&'a mut TcpStream);

impl io::Read for Reader<'_> {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(ErrorKind::WouldBlock, "needs async poll"))
    }
}

/// Sync writer adapter — always returns WouldBlock so the caller polls the async stream.
struct Writer<'a>(&'a mut TcpStream);

impl io::Write for Writer<'_> {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(ErrorKind::WouldBlock, "needs async poll"))
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl AsyncRead for TlsStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };

        // Drive handshake if needed
        if this.conn.is_handshaking() {
            match this.drive_handshake(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        // Try reading decrypted data from rustls
        loop {
            match this.conn.reader().read(buf) {
                Ok(n) => return Poll::Ready(Ok(n)),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // Need more TLS data from the underlying stream
                    match this.poll_read_tls(cx)? {
                        Poll::Ready(0) => return Poll::Ready(Ok(0)),
                        Poll::Ready(_) => {
                            match this.conn.process_new_packets() {
                                Ok(_) => continue, // retry reading decrypted data
                                Err(e) => return Poll::Ready(Err(io::Error::new(ErrorKind::InvalidData, e.to_string()))),
                            }
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.conn.is_handshaking() {
            match this.drive_handshake(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        // Write data to rustls writer
        let n = match this.conn.writer().write(buf) {
            Ok(n) => n,
            Err(e) => return Poll::Ready(Err(e)),
        };

        // Flush TLS records to underlying stream
        this.flush_tls(cx)?;

        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        this.flush_tls(cx)?;
        Pin::new(&mut this.stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        let _ = this.conn.send_close_notify();
        this.flush_tls(cx)?;
        Pin::new(&mut this.stream).poll_shutdown(cx)
    }
}

impl TlsStream {
    fn flush_tls(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            if !self.conn.wants_write() {
                return Poll::Ready(Ok(()));
            }
            match self.conn.write_tls(&mut Writer(&mut self.stream)) {
                Ok(_) => continue,
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    match Pin::new(&mut self.stream).poll_write(cx, &[]) {
                        Poll::Ready(Ok(_)) => continue,
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }
}

/// TLS acceptor future.
pub struct TlsAcceptFuture {
    stream: Option<TlsStream>,
}

impl Future for TlsAcceptFuture {
    type Output = io::Result<TlsStream>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let stream = this.stream.as_mut().unwrap();
        match stream.drive_handshake(cx) {
            Poll::Ready(Ok(())) => {
                let stream = this.stream.take().unwrap();
                Poll::Ready(Ok(stream))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// TLS acceptor — wraps a rustls ServerConfig.
pub struct TlsAcceptor {
    config: Arc<rustls::ServerConfig>,
}

impl TlsAcceptor {
    pub fn new(config: Arc<rustls::ServerConfig>) -> Self {
        Self { config }
    }

    pub fn accept(&self, stream: TcpStream) -> TlsAcceptFuture {
        let conn = rustls::ServerConnection::new(self.config.clone()).unwrap();
        TlsAcceptFuture { stream: Some(TlsStream::new(stream, conn)) }
    }
}

// Split TlsStream into read and write halves
pub struct TlsReadHalf<'a>(&'a mut TlsStream);
pub struct TlsWriteHalf<'a>(&'a mut TlsStream);

pub fn split(s: &mut TlsStream) -> (TlsReadHalf<'_>, TlsWriteHalf<'_>) {
    let ptr = s as *mut TlsStream;
    unsafe { (TlsReadHalf(&mut *ptr), TlsWriteHalf(&mut *ptr)) }
}

impl lifegraph_rt::AsyncRead for TlsReadHalf<'_> {
    fn poll_read(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &mut [u8]) -> std::task::Poll<std::io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        std::pin::Pin::new(&mut *this.0).poll_read(cx, buf)
    }
}

impl lifegraph_rt::AsyncWrite for TlsWriteHalf<'_> {
    fn poll_write(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &[u8]) -> std::task::Poll<std::io::Result<usize>> {
        let this = unsafe { self.get_unchecked_mut() };
        std::pin::Pin::new(&mut *this.0).poll_write(cx, buf)
    }
    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        std::pin::Pin::new(&mut *this.0).poll_flush(cx)
    }
    fn poll_shutdown(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        let this = unsafe { self.get_unchecked_mut() };
        std::pin::Pin::new(&mut *this.0).poll_shutdown(cx)
    }
}
