//! Runtime-facing aliases used by HTTP transport code.
//!
//! Protocol code should prefer these module paths over `std::...` so host and
//! bare-metal builds share one boundary for I/O, networking, time, filesystem,
//! synchronization, and collection compatibility.

#[cfg(feature = "std")]
pub use std::{collections, fs, io, net, path, sync, time};

#[cfg(not(feature = "std"))]
pub use crate::std_compat::{collections, fs, io, net, path, sync, time};

pub use edgerun_rt::sync::Mutex;
pub use edgerun_rt::{
    mpsc, oneshot, select, AsyncRead, AsyncReadExt, AsyncTcpListener, AsyncTcpStream,
    AsyncUdpSocket, AsyncWrite, AsyncWriteExt, BufReader, CancellationToken, ConnectFuture,
    JoinHandle,
};

/// Convert bare runtime I/O errors into the HTTP runtime I/O error type.
pub fn bare_io(error: edgerun_rt::IoError) -> io::Error {
    match error {
        edgerun_rt::IoError::UnexpectedEof => io::Error::new(io::ErrorKind::UnexpectedEof, error),
        edgerun_rt::IoError::WriteZero => io::Error::new(io::ErrorKind::WriteZero, error),
        edgerun_rt::IoError::Other(_) => io::Error::other(error),
    }
}

pub async fn sleep(delay: time::Duration) {
    edgerun_rt::sleep(delay).await;
}

pub fn timeout<F>(delay: time::Duration, future: F) -> edgerun_rt::Timeout<F>
where
    F: core::future::Future,
{
    edgerun_rt::timeout(delay, future)
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: core::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    edgerun_rt::spawn(future)
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    edgerun_rt::spawn_blocking(f)
}

pub fn bind_tcp_listener<A>(addr: A) -> io::Result<AsyncTcpListener>
where
    A: net::ToSocketAddrs,
{
    let addr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no socket address"))?;
    AsyncTcpListener::bind(addr).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub fn bind_udp_socket<A>(addr: A) -> io::Result<AsyncUdpSocket>
where
    A: net::ToSocketAddrs,
{
    let addr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no socket address"))?;
    AsyncUdpSocket::bind(addr).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

#[cfg(feature = "std")]
pub fn wrap_udp_socket(socket: net::UdpSocket) -> io::Result<AsyncUdpSocket> {
    AsyncUdpSocket::from_std(socket).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

#[cfg(not(feature = "std"))]
pub fn wrap_udp_socket(_socket: net::UdpSocket) -> io::Result<AsyncUdpSocket> {
    Err(io::Error::new(
        io::ErrorKind::Other,
        "wrapping host UDP sockets requires the std feature",
    ))
}
