#![cfg(all(feature = "std", feature = "http"))]

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::env;

use edgerun_node::rt::{
    self, AsyncRead, AsyncReadExt, AsyncTcpStream, AsyncWrite, AsyncWriteExt, ConnectFuture,
};
#[cfg(feature = "tls")]
use edgerun_node::tls::AsyncTlsStream;
use edgerun_work::{
    ChannelEnvelope, NODE_ROLE_MESSAGE, SimNode, WORK_WIRE_ABI_VERSION, WorkAck, WorkPacket,
    channel_envelope_bytes, channel_envelope_from_bytes, encode_work_packet_once,
};

extern crate alloc;

const DEFAULT_ADDR: &str = "203.0.113.10:80";
const DEFAULT_HOST: &str = "nodes.example.edgerun.local";
#[cfg(feature = "tls")]
const DEFAULT_WSS_ADDR: &str = "nodes.example.edgerun.local:443";

fn masked_client_binary(payload: &[u8]) -> Vec<u8> {
    let mask = [1u8, 2, 3, 4];
    let mut frame = Vec::new();
    frame.push(0x82);
    if payload.len() < 126 {
        frame.push(0x80 | payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(0x80 | 127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    frame.extend_from_slice(&mask);
    frame.extend(
        payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[index % mask.len()]),
    );
    frame
}

fn envelope(from_seed: u8, to_seed: u8, text: &str) -> ChannelEnvelope {
    let sender = SimNode::from_seed(from_seed, NODE_ROLE_MESSAGE);
    let recipient = SimNode::from_seed(to_seed, NODE_ROLE_MESSAGE);
    let packet = WorkPacket::Ack(WorkAck {
        ok: true,
        code: 200,
        text: text.to_owned(),
    });
    ChannelEnvelope {
        abi_version: WORK_WIRE_ABI_VERSION,
        channel_id: [3u8; 32],
        from: sender.identity.node_id,
        to: recipient.identity.node_id,
        route_hash: [4u8; 32],
        packet_hash: encode_work_packet_once(&packet)
            .expect("encoded packet")
            .hash,
        packet,
    }
}

async fn handshake_websocket<S>(mut stream: S, host: &str) -> Result<S, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let request = format!(
        "GET /work HTTP/1.1\r\n\
         Host: {host}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Protocol: edgerun-work-v1\r\n\
         \r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|e| format!("{e:?}"))?;

    let mut response = Vec::new();
    let mut chunk = [0u8; 256];
    while !response.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("{e:?}"))?;
        if read == 0 {
            return Err("server closed before WebSocket handshake completed".to_string());
        }
        response.extend_from_slice(&chunk[..read]);
        if response.len() > 4096 {
            return Err("WebSocket handshake response exceeded 4096 bytes".to_string());
        }
    }

    let response_text = String::from_utf8_lossy(&response);
    if !response_text.contains("101 Switching Protocols") {
        return Err(format!("unexpected handshake response: {response_text}"));
    }
    if !response_text.contains("Sec-WebSocket-Protocol: edgerun-work-v1") {
        return Err(format!("missing edgerun-work-v1 protocol: {response_text}"));
    }
    Ok(stream)
}

async fn connect_websocket(addr: &str, host: &str) -> Result<Arc<AsyncTcpStream>, String> {
    let stream = ConnectFuture::new(addr)
        .await
        .map_err(|e| format!("{e:?}"))?;
    handshake_websocket(stream, host).await
}

#[cfg(feature = "tls")]
async fn connect_secure_websocket(
    addr: &str,
    host: &str,
) -> Result<AsyncTlsStream<Arc<AsyncTcpStream>>, String> {
    let stream = ConnectFuture::new(addr)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let tls = AsyncTlsStream::client(stream, host, &[], None)
        .await
        .map_err(|e| format!("{e:?}"))?;
    handshake_websocket(tls, host).await
}

async fn write_envelope<S>(stream: &mut S, envelope: &ChannelEnvelope) -> Result<Vec<u8>, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let bytes = channel_envelope_bytes(envelope).map_err(|e| format!("{e:?}"))?;
    stream
        .write_all(&masked_client_binary(&bytes))
        .await
        .map_err(|e| format!("{e:?}"))?;
    stream.flush().await.map_err(|e| format!("{e:?}"))?;
    Ok(bytes)
}

async fn read_server_binary<S>(stream: &mut S) -> Result<Vec<u8>, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut header = [0u8; 2];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(header[0] & 0x0f, 0x2, "expected binary WebSocket frame");
    assert_eq!(header[1] & 0x80, 0, "server frames must not be masked");

    let mut length = (header[1] & 0x7f) as u64;
    if length == 126 {
        let mut extended = [0u8; 2];
        stream
            .read_exact(&mut extended)
            .await
            .map_err(|e| format!("{e:?}"))?;
        length = u16::from_be_bytes(extended) as u64;
    } else if length == 127 {
        let mut extended = [0u8; 8];
        stream
            .read_exact(&mut extended)
            .await
            .map_err(|e| format!("{e:?}"))?;
        length = u64::from_be_bytes(extended);
    }

    let mut payload = alloc::vec![0u8; length as usize];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(payload)
}

#[test]
#[ignore = "requires the live nodes.edgerun.tech origin WebSocket endpoint"]
fn live_origin_work_websocket_relays_between_two_identity_peers() {
    rt::block_on(async {
        let addr = env::var("EDGERUN_LIVE_WORK_WS_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.into());
        let host = env::var("EDGERUN_LIVE_WORK_WS_HOST").unwrap_or_else(|_| DEFAULT_HOST.into());

        let mut recipient = connect_websocket(&addr, &host)
            .await
            .expect("connect recipient WebSocket");
        let registration = envelope(2, 1, "recipient registers");
        write_envelope(&mut recipient, &registration)
            .await
            .expect("register recipient peer");

        let mut sender = connect_websocket(&addr, &host)
            .await
            .expect("connect sender WebSocket");
        let delivered = envelope(1, 2, "sender to recipient through live relay");
        let delivered_bytes = write_envelope(&mut sender, &delivered)
            .await
            .expect("send relay envelope");

        let received = read_server_binary(&mut recipient)
            .await
            .expect("recipient receives relayed frame");
        assert_eq!(received, delivered_bytes);
        assert_eq!(
            channel_envelope_from_bytes(&received).expect("decode relayed envelope"),
            delivered
        );
    });
}

#[cfg(feature = "tls")]
#[test]
#[ignore = "requires the live Cloudflare WSS endpoint for nodes.edgerun.tech"]
fn live_cloudflare_wss_relays_between_two_identity_peers() {
    rt::block_on(async {
        let addr =
            env::var("EDGERUN_LIVE_WORK_WSS_ADDR").unwrap_or_else(|_| DEFAULT_WSS_ADDR.into());
        let host = env::var("EDGERUN_LIVE_WORK_WS_HOST").unwrap_or_else(|_| DEFAULT_HOST.into());

        let mut recipient = connect_secure_websocket(&addr, &host)
            .await
            .expect("connect recipient WSS");
        let registration = envelope(2, 1, "recipient registers over wss");
        write_envelope(&mut recipient, &registration)
            .await
            .expect("register recipient WSS peer");

        let mut sender = connect_secure_websocket(&addr, &host)
            .await
            .expect("connect sender WSS");
        let delivered = envelope(1, 2, "sender to recipient through live wss relay");
        let delivered_bytes = write_envelope(&mut sender, &delivered)
            .await
            .expect("send WSS relay envelope");

        let received = read_server_binary(&mut recipient)
            .await
            .expect("recipient receives WSS relayed frame");
        assert_eq!(received, delivered_bytes);
        assert_eq!(
            channel_envelope_from_bytes(&received).expect("decode WSS relayed envelope"),
            delivered
        );
    });
}
