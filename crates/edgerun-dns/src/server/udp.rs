//! UDP receive loop — non-blocking DNS query handling over UDP
//! with rate limiting and graceful shutdown.

use alloc::sync::Arc;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::compat::AsyncUdpSocket;

use super::query::{handle_query, ParseError, ServerState};
use super::RateLimiter;
use crate::message::{DnsMessage, DnsResponseCode};

/// Run the UDP receive loop (async, runs until shutdown).
pub async fn udp_recv_loop_with_rate_limiting(
    socket: Arc<AsyncUdpSocket>,
    state: ServerState,
    rate_limiter: RateLimiter,
    shutdown: Arc<crate::compat::RwLock<bool>>,
) -> crate::std::io::Result<()> {
    loop {
        // Check shutdown flag
        if *shutdown.read().await {
            edgerun_log::info!("edgerun-dns: UDP loop shutting down");
            return Ok(());
        }

        let mut buf = [0u8; 4096];

        let (n, src) = match socket.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(e) => {
                edgerun_log::warn!("edgerun-dns: UDP recv error: {}", e);
                crate::compat::sleep(crate::std::time::Duration::from_millis(10)).await;
                continue;
            }
        };

        // Rate limit check
        if !rate_limiter.allow(src.ip()) {
            edgerun_log::debug!("edgerun-dns: rate limited query from {}", src.ip());
            // Send REFUSED response
            let response = DnsMessage::response(0, DnsResponseCode::Refused, Vec::new());
            let _ = socket.send_to(&response.to_wire(), src).await;
            continue;
        }

        let state = state.clone();
        let socket = Arc::clone(&socket);
        let query_buf = buf[..n].to_vec();

        crate::compat::spawn(async move {
            match handle_query(&query_buf, &state).await {
                Ok((response_wire, _needs_tcp)) => {
                    let _ = socket.send_to(&response_wire, src).await;
                }
                Err(ParseError) => {
                    let response = DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new());
                    let _ = socket.send_to(&response.to_wire(), src).await;
                }
            }
        });
    }
}

/// Legacy UDP loop (no rate limiting, no shutdown) — kept for backwards compat.
pub async fn udp_recv_loop(socket: Arc<AsyncUdpSocket>, state: ServerState) -> ! {
    let shutdown = Arc::new(crate::compat::RwLock::new(false));
    let rate_limiter = RateLimiter::new(0); // unlimited
    let _ = udp_recv_loop_with_rate_limiting(socket, state, rate_limiter, shutdown).await;
    unreachable!("UDP loop should never return without shutdown")
}
