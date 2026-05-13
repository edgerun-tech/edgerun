use alloc::format;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use crate::rt::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use edgerun_protocols::websocket::{
    WS_EXTENDED_64_LEN, WS_MASK_LEN, WebSocketError, WebSocketMessage, decode_client_message,
    decode_frame_prefix, decode_handshake_request, decode_payload_len, encode_server_binary,
    encode_server_control, encode_upgrade_response, handshake_complete,
};
use edgerun_work::{MAX_WORK_FRAME_LEN, NodeId, channel_envelope_from_bytes};

#[cfg(not(target_os = "none"))]
use std::{
    collections::BTreeMap,
    io,
    sync::mpsc::{self, SyncSender},
    sync::{Mutex, OnceLock},
};

#[cfg(not(target_os = "none"))]
const EDGERUN_WORK_WS_PROTOCOL: &str = "edgerun-work-v1";
#[cfg(not(target_os = "none"))]
const EDGERUN_WORK_WS_READ_POLL_MS: u64 = 100;
#[cfg(not(target_os = "none"))]
const EDGERUN_WORK_WS_MAX_QUEUE: usize = 256;

#[cfg(not(target_os = "none"))]
static EDGERUN_WORK_WS_PEERS: OnceLock<Mutex<BTreeMap<NodeId, SyncSender<Vec<u8>>>>> =
    OnceLock::new();

#[cfg(not(target_os = "none"))]
pub async fn serve_work_websocket<S>(mut stream: S, request: Vec<u8>) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let request = decode_handshake_request(request).map_err(invalid_ws_data)?;
    let response =
        encode_upgrade_response(&request, EDGERUN_WORK_WS_PROTOCOL).map_err(invalid_ws_data)?;
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(node_io_error)?;
    stream.flush().await.map_err(node_io_error)?;

    let peers = EDGERUN_WORK_WS_PEERS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(EDGERUN_WORK_WS_MAX_QUEUE);
    let mut registered_node = None;

    loop {
        let mut queued = Vec::new();
        while let Ok(frame) = rx.try_recv() {
            queued.push(frame);
        }
        for frame in queued {
            stream.write_all(&frame).await.map_err(node_io_error)?;
        }
        stream.flush().await.map_err(node_io_error)?;

        match rt::timeout(
            rt::Duration::from_millis(EDGERUN_WORK_WS_READ_POLL_MS),
            read_client_message(&mut stream),
        )
        .await
        {
            Ok(Ok(WebSocketMessage::Binary(bytes))) => {
                let envelope = channel_envelope_from_bytes(&bytes).map_err(invalid_ws_data)?;
                register_peer(peers, &mut registered_node, envelope.from, tx.clone());
                forward_envelope(peers, envelope.to, &bytes);
            }
            Ok(Ok(WebSocketMessage::Ping(payload))) => {
                let frame = encode_server_control(0xA, &payload).map_err(invalid_ws_data)?;
                stream.write_all(&frame).await.map_err(node_io_error)?;
                stream.flush().await.map_err(node_io_error)?;
            }
            Ok(Ok(WebSocketMessage::Pong)) => {}
            Ok(Ok(WebSocketMessage::Close)) => break,
            Ok(Err(error)) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Ok(Err(error)) => return Err(error),
            Err(_) => {}
        }
    }

    if let Some(node_id) = registered_node {
        peers
            .lock()
            .expect("work websocket peer map poisoned")
            .remove(&node_id);
    }
    Ok(())
}

#[cfg(not(target_os = "none"))]
pub async fn read_http_request<S>(stream: &mut S) -> io::Result<Vec<u8>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while request.len() <= edgerun_protocols::websocket::WS_MAX_HANDSHAKE_LEN {
        stream.read_exact(&mut byte).await.map_err(node_io_error)?;
        request.push(byte[0]);
        if handshake_complete(&request) {
            return Ok(request);
        }
    }
    Err(invalid_ws_data(WebSocketError::HandshakeTooLarge))
}

#[cfg(not(target_os = "none"))]
pub async fn write_http_response<S>(
    stream: &mut S,
    status: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.as_bytes().len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(node_io_error)?;
    stream.flush().await.map_err(node_io_error)
}

pub fn request_path(request: &str) -> Option<&str> {
    let mut parts = request.lines().next()?.split_whitespace();
    let _method = parts.next()?;
    parts.next()
}

pub fn request_method(request: &str) -> Option<&str> {
    request.lines().next()?.split_whitespace().next()
}

pub fn is_websocket_upgrade(request: &str) -> bool {
    request
        .lines()
        .any(|line| line.eq_ignore_ascii_case("upgrade: websocket"))
}

#[cfg(not(target_os = "none"))]
fn register_peer(
    peers: &Mutex<BTreeMap<NodeId, SyncSender<Vec<u8>>>>,
    registered_node: &mut Option<NodeId>,
    node_id: NodeId,
    tx: SyncSender<Vec<u8>>,
) {
    if registered_node.as_ref() == Some(&node_id) {
        return;
    }
    if let Some(previous) = registered_node.replace(node_id) {
        peers
            .lock()
            .expect("work websocket peer map poisoned")
            .remove(&previous);
    }
    peers
        .lock()
        .expect("work websocket peer map poisoned")
        .insert(node_id, tx);
}

#[cfg(not(target_os = "none"))]
fn forward_envelope(
    peers: &Mutex<BTreeMap<NodeId, SyncSender<Vec<u8>>>>,
    node_id: NodeId,
    payload: &[u8],
) {
    let Ok(frame) = encode_server_binary(payload).map_err(invalid_ws_data) else {
        return;
    };
    let mut peers = peers.lock().expect("work websocket peer map poisoned");
    let Some(tx) = peers.get(&node_id) else {
        return;
    };
    if tx.try_send(frame).is_err() {
        peers.remove(&node_id);
    }
}

#[cfg(not(target_os = "none"))]
async fn read_client_message<S>(stream: &mut S) -> io::Result<WebSocketMessage>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut header = [0u8; 2];
    stream
        .read_exact(&mut header)
        .await
        .map_err(node_io_error)?;
    let prefix = decode_frame_prefix(&header)
        .and_then(|prefix| prefix.require_client_mask())
        .map_err(invalid_ws_data)?;
    let mut extended = [0u8; WS_EXTENDED_64_LEN];
    let extended_len = prefix.extended_len_bytes();
    if extended_len != 0 {
        stream
            .read_exact(&mut extended[..extended_len])
            .await
            .map_err(node_io_error)?;
    }
    let payload_len = decode_payload_len(prefix, &extended[..extended_len], MAX_WORK_FRAME_LEN)
        .map_err(invalid_ws_data)?;
    let mut mask = [0u8; WS_MASK_LEN];
    stream.read_exact(&mut mask).await.map_err(node_io_error)?;
    let mut payload = vec![0u8; payload_len];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(node_io_error)?;
    decode_client_message(prefix.fin, prefix.opcode, mask, payload).map_err(invalid_ws_data)
}

#[cfg(not(target_os = "none"))]
fn invalid_ws_data(error: impl core::fmt::Debug) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid websocket data: {error:?}"),
    )
}

#[cfg(not(target_os = "none"))]
fn node_io_error(error: rt::IoError) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error.to_string())
}
