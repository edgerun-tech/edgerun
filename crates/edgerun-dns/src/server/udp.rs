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
                Ok((response_wire, needs_tcp)) => {
                    if needs_tcp {
                        if let Ok(query) = DnsMessage::from_wire(&query_buf) {
                            let mut response =
                                DnsMessage::response(query.header.id, DnsResponseCode::NoError, Vec::new());
                            response.header.truncated = true;
                            response.questions = query.questions;
                            response.header.question_count = response.questions.len() as u16;
                            let _ = socket.send_to(&response.to_wire(), src).await;
                        } else {
                            let _ = socket.send_to(&response_wire, src).await;
                        }
                    } else {
                        let _ = socket.send_to(&response_wire, src).await;
                    }
                }
                Err(ParseError) => {
                    let response = DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new());
                    let _ = socket.send_to(&response.to_wire(), src).await;
                }
            }
        });
    }
}
