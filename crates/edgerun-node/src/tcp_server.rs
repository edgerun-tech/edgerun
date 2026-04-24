use std::net::SocketAddr;
use std::sync::Arc;

use edgerun_hardware_signing::{MeshSigner, NodeID};
use prost::Message;

use crate::ingress;
use crate::session;
use crate::types::{StoreRequest, StoreResponse};

pub const TCP_MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Context for session handling -- holds node identity and signing capability.
#[derive(Clone)]
pub struct SessionContext {
    pub node_id: NodeID,
    pub signer: Arc<dyn MeshSigner + Send + Sync>,
}

pub async fn run_tcp_listener(
    listen_addr: SocketAddr,
    node_id: NodeID,
    store_tx: edgerun_rt::mpsc::Sender<StoreRequest>,
    signer: Arc<dyn MeshSigner + Send + Sync>,
    cancel: edgerun_rt::CancellationToken,
) {
    let listener = match edgerun_rt::AsyncTcpListener::bind(listen_addr) {
        Ok(l) => l,
        Err(e) => {
            edgerun_log::error!("failed to bind TCP on {}: {}", listen_addr, e);
            return;
        }
    };

    let ctx = SessionContext { node_id, signer };

    loop {
        if cancel.is_cancelled() {
            edgerun_log::info!("TCP listener shutting down");
            return;
        }
        let result = listener.accept().await;
        match result {
            Ok((stream, _peer_addr)) => {
                let conn_store_tx = store_tx.clone();
                let ctx = ctx.clone();
                edgerun_rt::spawn(async move {
                    edgerun_log::debug!("TCP connection accepted");
                    handle_tcp_connection(stream, conn_store_tx, &ctx, None).await;
                });
            }
            Err(e) => {
                edgerun_log::warn!("TCP accept error: {e}");
            }
        }
    }
}

/// Handles a plain TCP connection with session handshake.
/// If `outbound_nonce` is provided, this side initiates the handshake by
/// sending SessionHello first. Otherwise, it waits for the peer's hello.
pub async fn handle_tcp_connection(
    stream: Arc<edgerun_rt::AsyncTcpStream>,
    store_tx: edgerun_rt::mpsc::Sender<StoreRequest>,
    ctx: &SessionContext,
    outbound_nonce: Option<Vec<u8>>,
) {
    use edgerun_rt::AsyncReadExt;
    let (mut reader, mut writer) = stream.split();
    let mut read_buf = Vec::with_capacity(4096);
    let mut conn_rate_limiter = ingress::TokenBucket::new(100, 50);

    // Session handshake
    let session_state = if let Some(nonce) = outbound_nonce {
        // We're the initiator -- send hello first
        match perform_session_handshake_as_initiator(
            &mut reader,
            &mut writer,
            &mut read_buf,
            ctx,
            &nonce,
        ).await {
            Some(state) => {
                edgerun_log::info!("session established with peer (initiator)");
                state
            }
            None => {
                edgerun_log::debug!("session handshake failed as initiator");
                return;
            }
        }
    } else {
        // We're the responder -- wait for peer's hello
        match perform_session_handshake_as_responder(
            &mut reader,
            &mut writer,
            &mut read_buf,
            ctx,
        ).await {
            Some(state) => {
                edgerun_log::info!("session established with peer (responder)");
                state
            }
            None => {
                edgerun_log::debug!("session handshake failed as responder");
                return;
            }
        }
    };

    handle_tcp_stream_common_with_session(
        &mut reader,
        &mut writer,
        &mut read_buf,
        &mut conn_rate_limiter,
        &store_tx,
        &session_state,
    ).await;
}

/// Performs the session handshake as the initiator (client side).
/// Sends SessionHello first, then waits for SessionAccept.
pub async fn perform_session_handshake_as_initiator<R, W>(
    reader: &mut R,
    writer: &mut W,
    _read_buf: &mut Vec<u8>,
    ctx: &SessionContext,
    nonce: &[u8],
) -> Option<session::SessionState>
where
    R: edgerun_rt::AsyncRead + Unpin,
    W: edgerun_rt::AsyncWrite + Unpin,
{
    use edgerun_rt::{AsyncReadExt, AsyncWriteExt};

    // Build and send SessionHello
    let hello = match session::build_session_hello(
        &ctx.node_id,
        None, // No specific target
        nonce,
        &*ctx.signer,
    ) {
        Ok(h) => h,
        Err(e) => {
            edgerun_log::warn!("failed to build SessionHello: {}", e);
            return None;
        }
    };

    let hello_bytes = session::encode_hello(&hello);
    let frame = encode_tcp_frame(&hello_bytes);
    if writer.write_all(&frame).await.is_err() {
        return None;
    }
    if writer.flush().await.is_err() {
        return None;
    }

    // Read the response frame (should be SessionAccept)
    let mut resp_buf = Vec::with_capacity(4096);
    while resp_buf.len() < 8 {
        let mut chunk = [0u8; 64];
        match reader.read(&mut chunk).await {
            Ok(0) => return None,
            Ok(n) => resp_buf.extend_from_slice(&chunk[..n]),
            Err(_) => return None,
        }
    }

    let frame_len = match <[u8; 8]>::try_from(&resp_buf[..8]) {
        Ok(arr) => u64::from_be_bytes(arr) as usize,
        Err(_) => return None,
    };
    if frame_len == 0 || frame_len > TCP_MAX_FRAME_SIZE {
        return None;
    }

    let total_needed = 8 + frame_len;
    while resp_buf.len() < total_needed {
        let mut chunk = vec![0u8; 4096.min(total_needed - resp_buf.len())];
        match reader.read(&mut chunk).await {
            Ok(0) => return None,
            Ok(n) => resp_buf.extend_from_slice(&chunk[..n]),
            Err(_) => return None,
        }
    }

    let payload: Vec<u8> = resp_buf.drain(..total_needed).skip(8).collect();

    // Decode as SessionAccept
    let accept = match session::decode_accept(&payload) {
        Ok(a) => a,
        Err(_) => {
            edgerun_log::debug!("response is not a SessionAccept");
            return None;
        }
    };

    // Verify the SessionAccept
    let peer_node_id = match session::verify_session_accept(&accept, nonce) {
        Ok(id) => id,
        Err(e) => {
            edgerun_log::warn!("SessionAccept verification failed: {}", e);
            return None;
        }
    };

    let protocol_version = accept.selected_protocol_version;
    let transport_features = accept.selected_transport_features.clone();

    Some(session::SessionState {
        peer_node_id,
        peer_identity: accept.responder.clone()?,
        protocol_version,
        transport_features,
        is_initiator: true,
    })
}

/// Performs the session handshake as the responder (server side).
/// Waits for SessionHello, verifies it, sends SessionAccept.
async fn perform_session_handshake_as_responder<R, W>(
    reader: &mut R,
    writer: &mut W,
    read_buf: &mut Vec<u8>,
    ctx: &SessionContext,
) -> Option<session::SessionState>
where
    R: edgerun_rt::AsyncRead + Unpin,
    W: edgerun_rt::AsyncWrite + Unpin,
{
    use edgerun_rt::{AsyncReadExt, AsyncWriteExt};

    // Read the first frame (should be SessionHello)
    while read_buf.len() < 8 {
        let mut chunk = [0u8; 64];
        match reader.read(&mut chunk).await {
            Ok(0) => return None,
            Ok(n) => read_buf.extend_from_slice(&chunk[..n]),
            Err(_) => return None,
        }
    }

    let frame_len = match <[u8; 8]>::try_from(&read_buf[..8]) {
        Ok(arr) => u64::from_be_bytes(arr) as usize,
        Err(_) => return None,
    };
    if frame_len == 0 || frame_len > TCP_MAX_FRAME_SIZE {
        return None;
    }

    let total_needed = 8 + frame_len;
    while read_buf.len() < total_needed {
        let mut chunk = vec![0u8; 4096.min(total_needed - read_buf.len())];
        match reader.read(&mut chunk).await {
            Ok(0) => return None,
            Ok(n) => read_buf.extend_from_slice(&chunk[..n]),
            Err(_) => return None,
        }
    }

    let payload: Vec<u8> = read_buf.drain(..total_needed).skip(8).collect();

    // Try to decode as SessionHello
    let hello = match session::decode_hello(&payload) {
        Ok(h) => h,
        Err(_) => {
            edgerun_log::debug!("first frame is not a SessionHello");
            return None;
        }
    };

    // Verify the SessionHello
    let peer_node_id = match session::verify_session_hello(&hello) {
        Ok(id) => id,
        Err(e) => {
            edgerun_log::warn!("SessionHello verification failed: {}", e);
            return None;
        }
    };

    // Select protocol version and transport features
    let protocol_version = session::select_protocol_version(&hello.supported_protocol_versions)?;
    let transport_features = session::select_transport_features(&hello.supported_transport_features);

    // Build and send SessionAccept
    let nonce = hello.session_nonce.clone();
    let accept = match session::build_session_accept(
        &ctx.node_id,
        &nonce,
        protocol_version,
        transport_features.clone(),
        &*ctx.signer,
    ) {
        Ok(a) => a,
        Err(e) => {
            edgerun_log::warn!("failed to build SessionAccept: {}", e);
            return None;
        }
    };

    let accept_bytes = session::encode_accept(&accept);
    let frame = encode_tcp_frame(&accept_bytes);
    if writer.write_all(&frame).await.is_err() {
        return None;
    }
    if writer.flush().await.is_err() {
        return None;
    }

    edgerun_log::info!("session established");

    Some(session::SessionState {
        peer_node_id,
        peer_identity: hello.initiator.clone()?,
        protocol_version,
        transport_features,
        is_initiator: false,
    })
}

/// Common frame handling logic for TCP connections with session state.
async fn handle_tcp_stream_common_with_session<R, W>(
    reader: &mut R,
    writer: &mut W,
    read_buf: &mut Vec<u8>,
    conn_rate_limiter: &mut ingress::TokenBucket,
    store_tx: &edgerun_rt::mpsc::Sender<StoreRequest>,
    _session: &session::SessionState,
) where
    R: edgerun_rt::AsyncRead + Unpin,
    W: edgerun_rt::AsyncWrite + Unpin,
{
    use edgerun_rt::{AsyncReadExt, AsyncWriteExt};

    loop {
        // Read 8-byte length prefix
        while read_buf.len() < 8 {
            let mut chunk = [0u8; 64];
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    if read_buf.is_empty() {
                        return; // Clean close
                    }
                    edgerun_log::debug!("TCP closed mid-header");
                    return;
                }
                Ok(n) => {
                    read_buf.extend_from_slice(&chunk[..n]);
                }
                Err(e) => {
                    edgerun_log::debug!("TCP read error: {}", e);
                    return;
                }
            }
        }

        let frame_len = match <[u8; 8]>::try_from(&read_buf[..8]) {
            Ok(arr) => u64::from_be_bytes(arr) as usize,
            Err(_) => {
                edgerun_log::warn!("TCP frame length header invalid");
                return;
            }
        };
        if frame_len == 0 || frame_len > TCP_MAX_FRAME_SIZE {
            edgerun_log::warn!("TCP frame length invalid or too large");
            return;
        }

        // Read payload
        let total_needed = 8 + frame_len;
        while read_buf.len() < total_needed {
            let mut chunk = vec![0u8; 4096.min(total_needed - read_buf.len())];
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    edgerun_log::debug!("TCP closed mid-frame");
                    return;
                }
                Ok(n) => {
                    read_buf.extend_from_slice(&chunk[..n]);
                }
                Err(e) => {
                    edgerun_log::debug!("TCP read error: {}", e);
                    return;
                }
            }
        }

        let payload: Vec<u8> = {
            let frame = read_buf.drain(..total_needed).collect::<Vec<u8>>();
            frame.into_iter().skip(8).collect()
        };

        // Per-connection rate limit (cheap check, before decode or crypto)
        if !conn_rate_limiter.try_consume() {
            edgerun_log::warn!("TCP rate-limited connection");
            return;
        }

        // Try SessionHello (in case peer sends another hello)
        if let Ok(_hello) =
            edgerun_proto::edgerun::v0::network::SessionHello::decode(&payload[..])
        {
            edgerun_log::debug!("ignoring duplicate SessionHello");
            continue;
        }

        // Try CommandEnvelope
        if let Ok(command) =
            edgerun_proto::edgerun::v0::stream::CommandEnvelope::decode(&payload[..])
        {
            let raw = payload.clone();

            // Check if this is a snapshot publish command
            use edgerun_proto::edgerun::v0::stream::CommandType;
            if command.command_type == CommandType::PublishSnapshot as i32 {
                let (reply_tx, reply_rx) = edgerun_rt::oneshot::channel();
                if store_tx.send(StoreRequest::ProduceSnapshot {
                    view_type: "stream_heads".to_string(),
                    completeness: 1, // FULL
                    reply_tx,
                }).await.is_err() {
                    return;
                }
                match reply_rx.await {
                    Ok(StoreResponse::Ok(resp_payload)) => {
                        let resp_frame = encode_tcp_frame(&resp_payload);
                        if writer.write_all(&resp_frame).await.is_err() {
                            return;
                        }
                    }
                    Ok(StoreResponse::Rejected(_reason)) => {
                        edgerun_log::debug!("TCP snapshot production screened");
                        return;
                    }
                    Err(_) => return,
                }
                continue;
            }

            // Check if this is a fetch object command
            if command.command_type == CommandType::FetchObject as i32 {
                // Decode the raw payload as proto CommandEnvelope to get payload_object
                let proto_command = match edgerun_proto::edgerun::v0::stream::CommandEnvelope::decode(&raw[..]) {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        edgerun_log::warn!("FETCH_OBJECT: failed to decode proto command: {}", e);
                        let err = "FETCH_OBJECT: decode failed".to_string();
                        let resp_frame = encode_tcp_frame(err.as_bytes());
                        let _ = writer.write_all(&resp_frame).await;
                        continue;
                    }
                };

                // Extract the ObjectRef from the proto command's payload_object
                let object_ref = if let Some(ref obj) = proto_command.payload {
                    use edgerun_proto::edgerun::v0::stream::command_envelope::Payload;
                    match obj {
                        Payload::PayloadObject(obj) => obj.clone(),
                        Payload::InlinePayload(bytes) => {
                            if let Ok(obj) = edgerun_proto::edgerun::v0::common::ObjectRef::decode(&bytes[..]) {
                                obj
                            } else {
                                edgerun_proto::edgerun::v0::common::ObjectRef {
                                    object_id: vec![],
                                    object_kind: None,
                                }
                            }
                        }
                    }
                } else {
                    edgerun_proto::edgerun::v0::common::ObjectRef {
                        object_id: vec![],
                        object_kind: None,
                    }
                };

                if object_ref.object_id.is_empty() {
                    edgerun_log::warn!("FETCH_OBJECT: no object reference provided");
                    let err_resp = "FETCH_OBJECT: no object reference provided".to_string();
                    let resp_frame = encode_tcp_frame(err_resp.as_bytes());
                    let _ = writer.write_all(&resp_frame).await;
                    continue;
                }

                let (reply_tx, reply_rx) = edgerun_rt::oneshot::channel();
                if store_tx.send(StoreRequest::FetchObject { object_ref, reply_tx }).await.is_err() {
                    return;
                }
                match reply_rx.await {
                    Ok(StoreResponse::Ok(resp_payload)) => {
                        let resp_frame = encode_tcp_frame(&resp_payload);
                        if writer.write_all(&resp_frame).await.is_err() {
                            return;
                        }
                    }
                    Ok(StoreResponse::Rejected(_reason)) => {
                        edgerun_log::debug!("TCP fetch object screened");
                        return;
                    }
                    Err(_) => return,
                }
                continue;
            }

            let (reply_tx, reply_rx) = edgerun_rt::oneshot::channel();
            if store_tx.send(StoreRequest::Command {
                raw_bytes: raw, command, peer_id: None, reply_tx: Some(reply_tx),
            }).await.is_err() {
                return;
            }
            match reply_rx.await {
                Ok(StoreResponse::Ok(resp_payload)) => {
                    let resp_frame = encode_tcp_frame(&resp_payload);
                    if writer.write_all(&resp_frame).await.is_err() {
                        return;
                    }
                }
                Ok(StoreResponse::Rejected(_reason)) => {
                    edgerun_log::debug!("TCP message screened");
                    return;
                }
                Err(_) => return,
            }
            continue;
        }

        // Try QueryRequest
        if let Ok(query) =
            edgerun_proto::edgerun::v0::access::QueryRequest::decode(&payload[..])
        {
            let raw = payload.clone();
            let (reply_tx, reply_rx) = edgerun_rt::oneshot::channel();
            if store_tx.send(StoreRequest::Query {
                raw_bytes: raw, query, peer_id: None, reply_tx: Some(reply_tx),
            }).await.is_err() {
                return;
            }
            match reply_rx.await {
                Ok(StoreResponse::Ok(resp_payload)) => {
                    let resp_frame = encode_tcp_frame(&resp_payload);
                    if writer.write_all(&resp_frame).await.is_err() {
                        return;
                    }
                }
                Ok(StoreResponse::Rejected(_reason)) => {
                    edgerun_log::debug!("TCP query screened");
                    return;
                }
                Err(_) => return,
            }
            continue;
        }

        edgerun_log::debug!("TCP received unrecognized message type");
    }
}

pub fn encode_tcp_frame(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u64;
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}
