use std::collections::HashMap;
use std::sync::Arc;

use edgerun_encoding::byteorder::read_u64_be;
use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_rt::{AsyncReadExt, AsyncWriteExt, CancellationToken};
use prost::Message;

use crate::session::{self, SessionState};
use crate::tcp_server::{encode_tcp_frame, SessionContext, TCP_MAX_FRAME_SIZE};
use crate::types::StoreRequest;

/// Periodically attempts to reconnect to unreachable peers.
///
/// Runs on a timer, tracks unreachable peers and attempts TCP reconnection
/// with exponential backoff. On successful connection, runs the full session
/// handshake and message loop. Updates peer status in the store.
pub async fn run_peer_reconnection(
    initial_unreachable: Vec<(String, String)>,
    store_tx: edgerun_rt::mpsc::Sender<StoreRequest>,
    cancel: CancellationToken,
    signer: Arc<dyn MeshSigner + Send + Sync>,
    node_id: NodeID,
) {
    // Track backoff state per peer: (retry_count, next_attempt)
    let mut backoff: HashMap<String, (u32, std::time::Instant)> = HashMap::new();
    const INITIAL_BACKOFF_SECS: u64 = 5;
    const MAX_BACKOFF_SECS: u64 = 300; // 5 minutes

    // Initialize with known unreachable peers
    for (node_id_hex, addr) in initial_unreachable {
        backoff.insert(node_id_hex.clone(), (0, std::time::Instant::now()));
        let _ = store_tx
            .send(StoreRequest::PeerStatusUpdate {
                node_id_hex: node_id_hex.clone(),
                status: "unreachable".to_string(),
            })
            .await;
    }

    let mut interval = edgerun_rt::interval(std::time::Duration::from_secs(10));
    interval.set_missed_tick_behavior(edgerun_rt::MissedTickBehavior::Skip);

    loop {
        if cancel.is_cancelled() {
            edgerun_log::info!("peer reconnection shutting down");
            return;
        }
        interval.tick().await;

        let now = std::time::Instant::now();
        let mut to_remove = Vec::new();

        for (node_id_hex, (retry_count, next_attempt)) in backoff.iter_mut() {
            if now >= *next_attempt {
                let rc = *retry_count;
                let delay = (INITIAL_BACKOFF_SECS * 2u64.pow(rc.min(6))).min(MAX_BACKOFF_SECS);
                *next_attempt = now + std::time::Duration::from_secs(delay);
                *retry_count += 1;

                // Look up the peer's address from the store
                let peer_addr = match get_peer_addr(&store_tx, node_id_hex).await {
                    Some(addr) => addr,
                    None => {
                        edgerun_log::debug!("no address for peer {}, skipping", node_id_hex);
                        continue;
                    }
                };

                edgerun_log::info!(
                    "reconnecting to peer {} at {} (attempt {})",
                    node_id_hex,
                    peer_addr,
                    retry_count,
                );

                // Attempt TCP connection
                match edgerun_rt::ConnectFuture::new(&peer_addr).await {
                    Ok(stream) => {
                        edgerun_log::info!("connected to peer {}", node_id_hex);

                        // Update peer status to connected
                        let _ = store_tx
                            .send(StoreRequest::PeerStatusUpdate {
                                node_id_hex: node_id_hex.clone(),
                                status: "connected".to_string(),
                            })
                            .await;

                        // Perform session handshake as initiator
                        let nonce = session::generate_nonce();
                        let ctx = SessionContext {
                            node_id,
                            signer: signer.clone(),
                        };

                        match run_peer_session(stream, &store_tx, &ctx, &nonce).await {
                            Ok(()) => {
                                edgerun_log::info!("peer {} session ended cleanly", node_id_hex);
                                // Reset backoff on successful session
                                *retry_count = 0;
                                *next_attempt =
                                    now + std::time::Duration::from_secs(INITIAL_BACKOFF_SECS);
                            }
                            Err(e) => {
                                edgerun_log::warn!("peer {} session error: {}", node_id_hex, e);
                                // Update status back to unreachable
                                let _ = store_tx
                                    .send(StoreRequest::PeerStatusUpdate {
                                        node_id_hex: node_id_hex.clone(),
                                        status: "unreachable".to_string(),
                                    })
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        edgerun_log::warn!(
                            "failed to connect to peer {} at {}: {}",
                            node_id_hex,
                            peer_addr,
                            e,
                        );
                        // Update peer status to unreachable
                        let _ = store_tx
                            .send(StoreRequest::PeerStatusUpdate {
                                node_id_hex: node_id_hex.clone(),
                                status: "unreachable".to_string(),
                            })
                            .await;
                    }
                }

                // Clean up very old entries that keep failing
                if *retry_count > 20 {
                    to_remove.push(node_id_hex.clone());
                }
            }
        }

        for key in to_remove {
            backoff.remove(&key);
        }
    }
}

/// Look up a peer's address from the store.
async fn get_peer_addr(
    store_tx: &edgerun_rt::mpsc::Sender<StoreRequest>,
    node_id_hex: &str,
) -> Option<String> {
    use crate::types::StoreResponse;

    let (reply_tx, reply_rx) = edgerun_rt::oneshot::channel();
    store_tx
        .send(StoreRequest::PeerLookup {
            node_id_hex: node_id_hex.to_string(),
            reply_tx,
        })
        .await
        .ok()?;
    match reply_rx.await {
        Ok(StoreResponse::Ok(data)) if !data.is_empty() => String::from_utf8(data).ok(),
        _ => None,
    }
}

/// Run a full session with a peer: handshake then message loop.
async fn run_peer_session(
    stream: Arc<edgerun_rt::AsyncTcpStream>,
    store_tx: &edgerun_rt::mpsc::Sender<StoreRequest>,
    ctx: &SessionContext,
    nonce: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut reader, mut writer) = stream.split();

    // Perform session handshake as initiator
    let session_state = perform_peer_handshake(&mut reader, &mut writer, ctx, nonce).await?;

    // Run the message loop
    let mut rate_limiter = crate::ingress::TokenBucket::new(100, 50);
    let mut buf = Vec::with_capacity(4096);
    handle_peer_messages(
        &mut reader,
        &mut writer,
        &mut buf,
        &mut rate_limiter,
        store_tx,
        &session_state,
    )
    .await;

    Ok(())
}

/// Perform the session handshake as the initiator.
async fn perform_peer_handshake<R, W>(
    reader: &mut R,
    writer: &mut W,
    ctx: &SessionContext,
    nonce: &[u8],
) -> Result<SessionState, Box<dyn std::error::Error + Send + Sync>>
where
    R: edgerun_rt::AsyncRead + Unpin,
    W: edgerun_rt::AsyncWrite + Unpin,
{
    use edgerun_rt::AsyncWriteExt;

    // Build and send SessionHello
    let hello = session::build_session_hello(&ctx.node_id, None, nonce, &*ctx.signer)?;
    let hello_bytes = session::encode_hello(&hello);
    let frame = encode_tcp_frame(&hello_bytes);
    writer.write_all(&frame).await?;
    writer.flush().await?;

    // Read SessionAccept
    let mut resp_buf = Vec::with_capacity(4096);
    while resp_buf.len() < 8 {
        let mut chunk = [0u8; 64];
        match reader.read(&mut chunk).await? {
            0 => return Err("connection closed during handshake".into()),
            n => resp_buf.extend_from_slice(&chunk[..n]),
        }
    }

    let frame_len = read_u64_be(&resp_buf, 0) as usize;
    if frame_len == 0 || frame_len > crate::tcp_server::TCP_MAX_FRAME_SIZE {
        return Err("invalid frame length during handshake".into());
    }

    let total_needed = 8 + frame_len;
    while resp_buf.len() < total_needed {
        let mut chunk = vec![0u8; 4096.min(total_needed - resp_buf.len())];
        match reader.read(&mut chunk).await? {
            0 => return Err("connection closed during handshake".into()),
            n => resp_buf.extend_from_slice(&chunk[..n]),
        }
    }

    let payload: Vec<u8> = resp_buf.drain(..total_needed).skip(8).collect();
    let accept = session::decode_accept(&payload)?;
    let peer_node_id = session::verify_session_accept(&accept, nonce)?;

    let protocol_version = accept.selected_protocol_version;
    let transport_features = accept.selected_transport_features.clone();

    let peer_identity = accept.responder.clone().ok_or("no responder identity")?;

    Ok(SessionState {
        peer_node_id,
        peer_identity,
        protocol_version,
        transport_features,
        is_initiator: true,
    })
}

/// Run the message loop for a peer connection.
async fn handle_peer_messages<R, W>(
    reader: &mut R,
    writer: &mut W,
    read_buf: &mut Vec<u8>,
    rate_limiter: &mut crate::ingress::TokenBucket,
    store_tx: &edgerun_rt::mpsc::Sender<StoreRequest>,
    session: &SessionState,
) where
    R: edgerun_rt::AsyncRead + Unpin,
    W: edgerun_rt::AsyncWrite + Unpin,
{
    use edgerun_rt::AsyncReadExt;

    loop {
        // Read 8-byte length prefix
        while read_buf.len() < 8 {
            let mut chunk = [0u8; 64];
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    if read_buf.is_empty() {
                        return;
                    }
                    edgerun_log::debug!("peer connection closed mid-header");
                    return;
                }
                Ok(n) => {
                    read_buf.extend_from_slice(&chunk[..n]);
                }
                Err(e) => {
                    edgerun_log::debug!("peer read error: {}", e);
                    return;
                }
            }
        }

        let frame_len = read_u64_be(&read_buf, 0) as usize;
        if frame_len == 0 || frame_len > crate::tcp_server::TCP_MAX_FRAME_SIZE {
            edgerun_log::warn!("peer frame length invalid or too large");
            return;
        }

        // Read payload
        let total_needed = 8 + frame_len;
        while read_buf.len() < total_needed {
            let mut chunk = vec![0u8; 4096.min(total_needed - read_buf.len())];
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    edgerun_log::debug!("peer connection closed mid-frame");
                    return;
                }
                Ok(n) => {
                    read_buf.extend_from_slice(&chunk[..n]);
                }
                Err(e) => {
                    edgerun_log::debug!("peer read error: {}", e);
                    return;
                }
            }
        }

        let payload: Vec<u8> = {
            let frame = read_buf.drain(..total_needed).collect::<Vec<u8>>();
            frame.into_iter().skip(8).collect()
        };

        // Per-connection rate limit
        if !rate_limiter.try_consume() {
            edgerun_log::warn!("peer rate-limited connection");
            return;
        }

        // Try CommandEnvelope
        if let Ok(command) = edgerun_core::protocol::CommandEnvelope::decode(&payload[..]) {
            let raw = payload.clone();

            let (reply_tx, reply_rx) = edgerun_rt::oneshot::channel();
            if store_tx
                .send(StoreRequest::Command {
                    raw_bytes: raw,
                    command,
                    peer_id: None,
                    reply_tx: Some(reply_tx),
                })
                .await
                .is_err()
            {
                return;
            }
            match reply_rx.await {
                Ok(crate::types::StoreResponse::Ok(resp_payload)) => {
                    let resp_frame = encode_tcp_frame(&resp_payload);
                    if writer.write_all(&resp_frame).await.is_err() {
                        return;
                    }
                }
                Ok(crate::types::StoreResponse::Rejected(_reason)) => {
                    edgerun_log::debug!("peer message screened");
                    return;
                }
                Err(_) => return,
            }
            continue;
        }

        // Try QueryRequest
        if let Ok(query) = edgerun_core::protocol::QueryRequest::decode(&payload[..]) {
            let raw = payload.clone();
            let (reply_tx, reply_rx) = edgerun_rt::oneshot::channel();
            if store_tx
                .send(StoreRequest::Query {
                    raw_bytes: raw,
                    query,
                    peer_id: None,
                    reply_tx: Some(reply_tx),
                })
                .await
                .is_err()
            {
                return;
            }
            match reply_rx.await {
                Ok(crate::types::StoreResponse::Ok(resp_payload)) => {
                    let resp_frame = encode_tcp_frame(&resp_payload);
                    if writer.write_all(&resp_frame).await.is_err() {
                        return;
                    }
                }
                Ok(crate::types::StoreResponse::Rejected(_reason)) => {
                    edgerun_log::debug!("peer query screened");
                    return;
                }
                Err(_) => return,
            }
            continue;
        }

        edgerun_log::debug!("peer received unrecognized message type");
    }
}
