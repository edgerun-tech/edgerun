//! TCP accept loop and connection handler — length-prefixed DNS over TCP
//! with rate limiting and graceful shutdown.

use crate::std::io;
use crate::std::net::SocketAddr;
use alloc::sync::Arc;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::future::poll_fn;
use core::pin::Pin;

use crate::compat::AsyncRead;
use crate::compat::AsyncTcpListener;
use crate::compat::AsyncTcpStream;
use crate::compat::AsyncWrite;

use super::query::{handle_query, ParseError, ServerState};
use super::RateLimiter;
use crate::message::{DnsMessage, DnsResponseCode};
use crate::tcp_frame::{dns_tcp_frame_len, encode_dns_tcp_frame};

/// Run the TCP accept loop — spawns a handler for each connection.
/// Runs until shutdown is requested.
pub async fn tcp_accept_loop_with_shutdown(
    listener: Arc<AsyncTcpListener>,
    state: ServerState,
    rate_limiter: RateLimiter,
    shutdown: Arc<crate::compat::RwLock<bool>>,
) {
    loop {
        match listener.accept_until_shutdown(&shutdown).await {
            Ok(Some((stream, peer))) => {
                let state = state.clone();
                let rate_limiter = rate_limiter.clone();
                crate::compat::spawn(async move {
                    if let Err(e) =
                        handle_tcp_connection_raw(stream, peer, &state, &rate_limiter).await
                    {
                        edgerun_log::warn!("edgerun-dns: TCP error from {}: {}", peer, e);
                    }
                });
            }
            Ok(None) => {
                edgerun_log::info!("edgerun-dns: TCP loop shutting down");
                return;
            }
            Err(e) => {
                edgerun_log::warn!("edgerun-dns: TCP accept error: {}", e);
                crate::compat::sleep(crate::std::time::Duration::from_millis(10)).await;
            }
        }
    }
}

/// Handle a single TCP connection with length-prefixed DNS messages.
/// Public raw version — used by both TCP and DoT servers.
pub async fn handle_tcp_connection_raw(
    stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    state: &ServerState,
    rate_limiter: &RateLimiter,
) -> Result<(), io::Error> {
    edgerun_log::debug!("edgerun-dns: TCP connection from {}", peer);

    let stream_mutex = Arc::new(crate::std::sync::Mutex::new(stream));

    loop {
        // Read 2-byte length prefix.
        let mut len_buf = [0u8; 2];
        match tcp_read_exact(&stream_mutex, &mut len_buf).await {
            Ok(0) => return Ok(()),
            Ok(2) => {}
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "incomplete TCP length",
                ));
            }
            Err(e) => return Err(e),
        }
        let msg_len = dns_tcp_frame_len(len_buf).map_err(dns_tcp_frame_io_error)?;

        let mut query_buf = vec![0u8; msg_len];
        tcp_read_exact(&stream_mutex, &mut query_buf).await?;

        // Rate limit per source IP
        if !rate_limiter.allow(peer.ip()) {
            edgerun_log::debug!("edgerun-dns: rate limited TCP query from {}", peer.ip());
            let response = DnsMessage::response(0, DnsResponseCode::Refused, Vec::new());
            tcp_write_length_prefixed(&stream_mutex, &response.to_wire()).await?;
            continue;
        }

        match handle_query(&query_buf, state).await {
            Ok((response_wire, _tcp_needed)) => {
                tcp_write_length_prefixed(&stream_mutex, &response_wire).await?;
            }
            Err(ParseError) => {
                let response = DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new());
                tcp_write_length_prefixed(&stream_mutex, &response.to_wire()).await?;
            }
        }
    }
}

/// Read exactly `n` bytes from a TCP stream behind a Mutex<Arc>.
async fn tcp_read_exact(
    stream_mutex: &Arc<crate::std::sync::Mutex<Arc<AsyncTcpStream>>>,
    buf: &mut [u8],
) -> io::Result<usize> {
    let mut total = 0;
    let n = buf.len();
    while total < n {
        let read = poll_fn(|cx| {
            let guard = stream_mutex.lock().unwrap();
            let stream_ptr = Arc::as_ptr(&guard) as *mut AsyncTcpStream;
            let stream_mut = unsafe { &mut *stream_ptr };
            Pin::new(stream_mut).poll_read(cx, &mut buf[total..n])
        })
        .await?;

        if read == 0 {
            return if total == 0 {
                Ok(0)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "incomplete TCP read",
                ))
            };
        }
        total += read;
    }
    Ok(total)
}

/// Write a length-prefixed DNS response over TCP.
async fn tcp_write_length_prefixed(
    stream_mutex: &Arc<crate::std::sync::Mutex<Arc<AsyncTcpStream>>>,
    data: &[u8],
) -> io::Result<()> {
    let frame = encode_dns_tcp_frame(data).map_err(dns_tcp_frame_io_error)?;

    poll_fn(|cx| {
        let guard = stream_mutex.lock().unwrap();
        let stream_ptr = Arc::as_ptr(&guard) as *mut AsyncTcpStream;
        let stream_mut = unsafe { &mut *stream_ptr };
        Pin::new(stream_mut).poll_write(cx, &frame[..2])
    })
    .await?;

    let mut written = 0;
    let data = &frame[2..];
    while written < data.len() {
        let n = poll_fn(|cx| {
            let guard = stream_mutex.lock().unwrap();
            let stream_ptr = Arc::as_ptr(&guard) as *mut AsyncTcpStream;
            let stream_mut = unsafe { &mut *stream_ptr };
            Pin::new(stream_mut).poll_write(cx, &data[written..])
        })
        .await?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "TCP write zero"));
        }
        written += n;
    }

    Ok(())
}

fn dns_tcp_frame_io_error(err: crate::tcp_frame::DnsTcpFrameError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, alloc::format!("{err:?}"))
}
