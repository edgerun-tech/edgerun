//! UDP receive loop — non-blocking DNS query handling over UDP.

use std::sync::Arc;

use edgerun_rt::AsyncUdpSocket;

use crate::message::{DnsMessage, DnsResponseCode};
use super::query::{handle_query, ParseError, ServerState};

/// Run the UDP receive loop (async, runs forever).
pub async fn udp_recv_loop(socket: Arc<AsyncUdpSocket>, state: ServerState) -> ! {
    loop {
        let mut buf = [0u8; 4096];

        let (n, src) = match socket.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(e) => {
                edgerun_log::warn!("edgerun-dns: UDP recv error: {}", e);
                continue;
            }
        };

        let state = state.clone();
        let socket = Arc::clone(&socket);
        let query_buf = buf[..n].to_vec();

        edgerun_rt::spawn(async move {
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
